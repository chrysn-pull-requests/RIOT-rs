//! Experimental types for ACE, COSE and CWT structures
//!
//! On the long run, those might contribute to
//! <https://github.com/namib-project/dcaf-rs/issues/29>.
//!
//! The module is private, but contains a few pub items so that they can be used in the
//! [`seccfg`][crate::seccfg] crate on sealed traits.

use defmt_or_log::trace;

use crate::error::{CredentialError, CredentialErrorDetail};

use crate::helpers::COwn;
use crate::time::TimeConstraint;

/// Fixed length of the ACE OSCORE nonce issued by this module.
pub(crate) const OWN_NONCE_LEN: usize = 8;

/// Size allocated for the ACE OSCORE nonces chosen by the peers.
const MAX_SUPPORTED_PEER_NONCE_LEN: usize = 16;

/// Maximum size a CWT processed by this module can have (at least when it needs to be copied)
const MAX_SUPPORTED_ACCESSTOKEN_LEN: usize = 256;
/// Maximum size of a COSE_Encrypt0 protected header (used to size the AAD buffer)
const MAX_SUPPORTED_ENCRYPT_PROTECTED_LEN: usize = 32;

/// The content of an application/ace+cbor file.
///
/// Full attribute references are in the [OAuth Parameters CBOR Mappings
/// registry](https://www.iana.org/assignments/ace/ace.xhtml#oauth-parameters-cbor-mappings).
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode, minicbor::Encode, Default)]
#[cbor(map)]
#[non_exhaustive]
struct AceCbor<'a> {
    #[cbor(b(1), with = "minicbor::bytes")]
    access_token: Option<&'a [u8]>,
    #[cbor(b(40), with = "minicbor::bytes")]
    nonce1: Option<&'a [u8]>,
    #[cbor(b(42), with = "minicbor::bytes")]
    nonce2: Option<&'a [u8]>,
    #[cbor(b(43), with = "minicbor::bytes")]
    ace_client_recipientid: Option<&'a [u8]>,
    #[cbor(b(44), with = "minicbor::bytes")]
    ace_server_recipientid: Option<&'a [u8]>,
}

/// The content of a POST to the /authz-info endpoint of a client.
///
/// # Open questions
/// Should we subset the type to add more constraints on fields?
///
/// * Pro type alias: Shared parsing code for all cases.
/// * Pro subtype: Easier usability, errors directly from minicbor.
type UnprotectedAuthzInfoPost<'a> = AceCbor<'a>;

/// A COSE header map.
///
/// Full attribute references are in the [COSE Header Parameters
/// registry](https://www.iana.org/assignments/cose/cose.xhtml#header-parameters).
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode)]
#[cbor(map)]
#[non_exhaustive]
pub struct HeaderMap<'a> {
    #[n(1)]
    // Might be extended as more exotic algorithms are supported
    pub(crate) alg: Option<i32>,
    #[cbor(b(5), with = "minicbor::bytes")]
    pub(crate) iv: Option<&'a [u8]>,
}

impl HeaderMap<'_> {
    /// Merge two header maps, using the latter's value in case of conflict.
    fn updated_with(&self, other: Self) -> Self {
        Self {
            alg: self.alg.or(other.alg),
            iv: self.iv.or(other.iv),
        }
    }
}

/// A COSE_Key as described in Section 7 of RFC9052.
///
/// This combines [COSE Key Common
/// Parameters](https://www.iana.org/assignments/cose/cose.xhtml#key-common-parameters) with [COSE
/// Key Type Parameters](https://www.iana.org/assignments/cose/cose.xhtml#key-type-parameters)
/// under the assumption that the key type is 1 (OKP) or 2 (EC2), which so far have non-conflicting
/// entries.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode, Debug)]
#[allow(
    dead_code,
    reason = "Presence of the item makes CBOR derive tolerate the item"
)]
#[cbor(map)]
#[non_exhaustive]
pub(crate) struct CoseKey<'a> {
    #[n(1)]
    pub(crate) kty: i32, // or tstr (unsupported here so far)
    #[cbor(b(2), with = "minicbor::bytes")]
    pub(crate) kid: Option<&'a [u8]>,
    #[n(3)]
    pub(crate) alg: Option<i32>, // or tstr (unsupported here so far)

    #[n(-1)]
    pub(crate) crv: Option<i32>, // or tstr (unsupported here so far)
    #[cbor(b(-2), with = "minicbor::bytes")]
    pub(crate) x: Option<&'a [u8]>,
    #[cbor(b(-3), with = "minicbor::bytes")]
    pub(crate) y: Option<&'a [u8]>, // or bool (unsupported here so far)
}

/// A COSE_Encrypt0 structure as defined in [RFC8152](https://www.rfc-editor.org/rfc/rfc8152)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode)]
#[cbor(tag(16))]
#[non_exhaustive]
struct CoseEncrypt0<'a> {
    #[cbor(b(0), with = "minicbor::bytes")]
    protected: &'a [u8],
    #[b(1)]
    unprotected: HeaderMap<'a>,
    #[cbor(b(2), with = "minicbor::bytes")]
    encrypted: &'a [u8],
}

impl CoseEncrypt0<'_> {
    /// Performs the common steps of processing the inner headers and building an AAD before
    /// passing the output on to an authority's `.decrypt_symmetric_token` method.
    ///
    /// The buffer could be initialized anew and place-returned, but as it is large, it is taken as
    /// a reference so that (eg. in `process_edhoc_token`) it can be guaranteed to be shared with
    /// the large buffer of the other path.
    fn prepare_decryption<'t>(
        &self,
        buffer: &'t mut heapless::Vec<u8, MAX_SUPPORTED_ACCESSTOKEN_LEN>,
    ) -> Result<(HeaderMap<'_>, impl AsRef<[u8]>, &'t mut [u8]), CredentialError> {
        trace!("Preparing decryption of {:?}", self);

        // Could have the extra exception for empty byte strings expressing the empty map, but we don't
        // encounter this here
        let protected: HeaderMap = minicbor::decode(self.protected)?;
        trace!("Protected decoded as header map: {:?}", protected);
        let headers = self.unprotected.updated_with(protected);

        #[derive(minicbor::Encode)]
        struct Encrypt0<'a> {
            #[n(0)]
            context: &'static str,
            #[cbor(b(1), with = "minicbor::bytes")]
            protected: &'a [u8],
            #[cbor(b(2), with = "minicbor::bytes")]
            external_aad: &'a [u8],
        }
        let aad = Encrypt0 {
            context: "Encrypt0",
            protected: self.protected,
            external_aad: &[],
        };
        const AADSIZE: usize = 1 + 1 + 8 + 1 + MAX_SUPPORTED_ENCRYPT_PROTECTED_LEN + 1;
        let mut aad_encoded = heapless::Vec::<u8, AADSIZE>::new();
        minicbor::encode(&aad, minicbor_adapters::WriteToHeapless(&mut aad_encoded))
            .map_err(|_| CredentialErrorDetail::ConstraintExceeded)?;
        trace!("Serialized AAD: {:02x}", aad_encoded); // :02x could be :cbor

        buffer.clear();
        // Copying around is not a constraint of this function (well that too but that could
        // change) -- but the callers don't usually get their data in a mutable buffer for in-place
        // decryption.
        buffer
            .extend_from_slice(self.encrypted)
            .map_err(|_| CredentialErrorDetail::ConstraintExceeded)?;

        Ok((headers, aad_encoded, buffer))
    }
}

type EncryptedCwt<'a> = CoseEncrypt0<'a>;

/// A COSE_Sign1 structure as defined in [RFC8152](https://www.rfc-editor.org/rfc/rfc8152)
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode)]
#[cbor(tag(18))]
#[non_exhaustive]
struct CoseSign1<'a> {
    #[cbor(b(0), with = "minicbor::bytes")]
    protected: &'a [u8],
    #[b(1)]
    unprotected: HeaderMap<'a>,
    // Payload could also be nil, but we don't support detached signatures here right now.
    #[cbor(b(2), with = "minicbor::bytes")]
    payload: &'a [u8],
    #[cbor(b(3), with = "minicbor::bytes")]
    signature: &'a [u8],
}

type SignedCwt<'a> = CoseSign1<'a>;

/// A CWT Claims Set.
///
/// Full attribute references are in the [CWT Claims
/// registry](https://www.iana.org/assignments/cwt/cwt.xhtml#claims-registry).
#[derive(minicbor::Decode, Debug)]
#[allow(
    dead_code,
    reason = "Presence of the item makes CBOR derive tolerate the item"
)]
#[cbor(map)]
#[non_exhaustive]
pub struct CwtClaimsSet<'a> {
    #[n(3)]
    pub(crate) aud: Option<&'a str>,
    #[n(4)]
    pub(crate) exp: u64,
    #[n(6)]
    pub(crate) iat: u64,
    #[b(8)]
    cnf: Cnf<'a>,
    #[cbor(b(9), with = "minicbor::bytes")]
    pub(crate) scope: &'a [u8],
}

/// A single CWT Claims Set Confirmation value.
///
/// All possible variants are in the [CWT Confirmation Methods
/// registry](https://www.iana.org/assignments/cwt/cwt.xhtml#confirmation-methods).
///
/// ## Open questions
///
/// This should be an enum, but minicbor-derive can only have them as `array(2)` or using
/// `index_only`. Can this style of an enum be added to minicbor?
///
/// Or is it really an enum? RFC8747 just [talks
/// of](https://www.rfc-editor.org/rfc/rfc8747.html#name-confirmation-claim) "At most one of the
/// `COSE_Key` and `Encrypted_COSE_Key` […] may be present", doesn't rule out that items without
/// key material can't be attached.
#[derive(minicbor::Decode, Debug)]
#[cbor(map)]
#[non_exhaustive]
struct Cnf<'a> {
    #[b(4)]
    osc: Option<OscoreInputMaterial<'a>>,
    #[b(1)]
    cose_key: Option<minicbor_adapters::WithOpaque<'a, CoseKey<'a>>>,
}

/// OSCORE_Input_Material.
///
/// All current parameters are described in [Section 3.2.1 of
/// RFC9203](https://datatracker.ietf.org/doc/html/rfc9203#name-the-oscore_input_material); the
/// [OSCORE Security Context Parameters
/// registry](https://www.iana.org/assignments/ace/ace.xhtml#oscore-security-context-parameters)
/// has the full set in case it gets extended.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(minicbor::Decode, Debug)]
#[allow(
    dead_code,
    reason = "Presence of the item makes CBOR derive tolerate the item"
)]
#[cbor(map)]
#[non_exhaustive]
struct OscoreInputMaterial<'a> {
    #[cbor(b(0), with = "minicbor::bytes")]
    id: &'a [u8],
    #[cbor(b(2), with = "minicbor::bytes")]
    ms: &'a [u8],
}

impl OscoreInputMaterial<'_> {
    fn derive(
        &self,
        nonce1: &[u8],
        nonce2: &[u8],
        sender_id: &[u8],
        recipient_id: &[u8],
    ) -> Result<liboscore::PrimitiveContext, CredentialError> {
        // We don't process the algorithm fields
        let hkdf = liboscore::HkdfAlg::from_number(5).expect("Default algorithm is supported");
        let aead = liboscore::AeadAlg::from_number(10).expect("Default algorithm is supported");

        // This is the only really custom part of ACE-OSCORE; the rest is just passing around
        // inputs.
        const { assert!(OWN_NONCE_LEN < 256) };
        const { assert!(MAX_SUPPORTED_PEER_NONCE_LEN < 256) };
        let mut combined_salt =
            heapless::Vec::<u8, { 1 + 2 + MAX_SUPPORTED_PEER_NONCE_LEN + 2 + OWN_NONCE_LEN }>::new(
            );
        let mut encoder =
            minicbor::Encoder::new(minicbor_adapters::WriteToHeapless(&mut combined_salt));
        // We don't process the salt field
        encoder
            .bytes(b"")
            .and_then(|encoder| encoder.bytes(nonce1))
            .and_then(|encoder| encoder.bytes(nonce2))?;

        let immutables = liboscore::PrimitiveImmutables::derive(
            hkdf,
            self.ms,
            &combined_salt,
            None, // context ID field not processed
            aead,
            sender_id,
            recipient_id,
        )
        // Unknown HKDF is probably the only case here.
        .map_err(|_| CredentialErrorDetail::UnsupportedAlgorithm)?;

        // It is fresh because it is derived from.
        Ok(liboscore::PrimitiveContext::new_from_fresh_material(
            immutables,
        ))
    }
}

/// An owned variety of the subset of [`AceCbor`] data.
///
/// It needs a slim owned form that is kept by the server between processing an ACE-OSCORE token
/// POST request and sending the response, and conveniently encapsulates its own rendering into a
/// response message.
pub struct AceCborAuthzInfoResponse {
    nonce2: [u8; OWN_NONCE_LEN],
    ace_server_recipientid: COwn,
}

impl AceCborAuthzInfoResponse {
    pub(crate) fn render<M: coap_message::MutableWritableMessage>(
        &self,
        message: &mut M,
    ) -> Result<(), M::UnionError> {
        let full = AceCbor {
            nonce2: Some(&self.nonce2),
            ace_server_recipientid: Some(self.ace_server_recipientid.as_slice()),
            ..Default::default()
        };

        use coap_message::Code;
        message.set_code(M::Code::new(coap_numbers::code::CHANGED)?);

        const { assert!(OWN_NONCE_LEN < 256) };
        const { assert!(COwn::MAX_SLICE_LEN < 256) };
        let required_len = 1 + 2 + 2 + OWN_NONCE_LEN + 2 + 2 + COwn::MAX_SLICE_LEN;
        let payload = message.payload_mut_with_len(required_len)?;

        let mut cursor = minicbor::encode::write::Cursor::new(payload);
        minicbor::encode(full, &mut cursor).expect("Sufficient size was requested");
        let written = cursor.position();
        message.truncate(written)?;

        Ok(())
    }
}

/// Given an application/ace+cbor payload as is posted to an /authz-info endpoint, decrypt all
/// that's needed for the ACE-OSCORE profile.
///
/// This needs to be provided with
///
/// * the request's `payload`
/// * a list of recognized `authorities` (Authorization Servers) to authenticate the token,
///   the output of which is also later used to parse the token's scope.
/// * a random nonce2
/// * a callback that, once the peer's recipient ID is known, chooses an own recipient ID
///   (because it's up to the pool of security contexts to pick one, and the peers can not pick
///   identical ones)
///
/// ## Caveats
///
/// * This allocates on the stack for two fields: the AAD and the token's plaintext. Both will
///   eventually need to be configurable.
///
///   Alternatives to allocation are streaming AADs for the AEAD traits, and coap-handler offering
///   an exclusive reference to the incoming message.
///
/// * Instead of the random nonce2, it would be preferable to pass in an RNG -- but some owners of
///   an RNG may have a hard time lending out an exclusive reference to it for the whole function
///   call duration.
pub(crate) fn process_acecbor_authz_info<Scope>(
    payload: &[u8],
    authorities: &impl crate::seccfg::ServerSecurityConfig<Scope = Scope>,
    nonce2: [u8; OWN_NONCE_LEN],
    server_recipient_id: impl FnOnce(&[u8]) -> COwn,
) -> Result<
    (
        AceCborAuthzInfoResponse,
        liboscore::PrimitiveContext,
        Scope,
        crate::time::TimeConstraint,
    ),
    CredentialError,
> {
    trace!("Processing authz_info {=[u8]:02x}", payload); // :02x could be :cbor

    let decoded: UnprotectedAuthzInfoPost = minicbor::decode(payload)?;
    // FIXME: The `..` should be "all others are None"; se also comment on UnprotectedAuthzInfoPost
    // on type alias vs new type
    let AceCbor {
        access_token: Some(access_token),
        nonce1: Some(nonce1),
        ace_client_recipientid: Some(ace_client_recipientid),
        ..
    } = decoded
    else {
        return Err(CredentialErrorDetail::ProtocolViolation.into());
    };

    trace!(
        "Decodeded authz_info as application/ace+cbor: {:?}",
        decoded
    );

    let encrypt0: EncryptedCwt = minicbor::decode(access_token)?;

    let mut buffer = Default::default();
    let (headers, aad_encoded, buffer) = encrypt0.prepare_decryption(&mut buffer)?;

    // Can't go through liboscore's decryption backend b/c that expects unprotect-in-place; doing
    // something more custom on a bounded copy instead, and this is part of where dcaf on alloc
    // could shine by getting an exclusive copy of something in RAM

    if headers.alg != Some(31) {
        return Err(CredentialErrorDetail::UnsupportedAlgorithm.into());
    }

    let (scope, claims) = authorities.decrypt_symmetric_token(
        &headers,
        aad_encoded.as_ref(),
        buffer,
        crate::PrivateMethod,
    )?;

    // Currently disabled because no formatting is available while there; works with
    // <https://codeberg.org/chrysn/minicbor-adapters/pulls/1>
    // trace!("Decrypted CWT claims: {}", claims);

    let time_constraint = TimeConstraint::from_claims_set(&claims);

    let Cnf {
        osc: Some(osc),
        cose_key: None,
    } = claims.cnf
    else {
        return Err(CredentialErrorDetail::InconsistentDetails.into());
    };

    let ace_server_recipientid = server_recipient_id(ace_client_recipientid);

    let derived = osc.derive(
        nonce1,
        &nonce2,
        ace_client_recipientid,
        ace_server_recipientid.as_slice(),
    )?;

    let response = AceCborAuthzInfoResponse {
        nonce2,
        ace_server_recipientid,
    };

    Ok((response, derived, scope, time_constraint))
}

pub(crate) fn process_edhoc_token<Scope>(
    ead3: &[u8],
    authorities: &impl crate::seccfg::ServerSecurityConfig<Scope = Scope>,
) -> Result<(lakers::Credential, Scope, TimeConstraint), CredentialError> {
    let mut buffer = heapless::Vec::<u8, MAX_SUPPORTED_ACCESSTOKEN_LEN>::new();

    // Trying and falling back means that the minicbor error is not too great ("Expected tag 16"
    // rather than "Expected tag 16 or 18"), but we don't
    // show much of that anyway.
    let (scope, claims) = if let Ok(encrypt0) = minicbor::decode::<EncryptedCwt>(ead3) {
        let (headers, aad_encoded, buffer) = encrypt0.prepare_decryption(&mut buffer)?;

        authorities.decrypt_symmetric_token(
            &headers,
            aad_encoded.as_ref(),
            buffer,
            crate::PrivateMethod,
        )?
    } else if let Ok(sign1) = minicbor::decode::<SignedCwt>(ead3) {
        let protected: HeaderMap = minicbor::decode(sign1.protected)?;
        trace!(
            "Decoded protected header map {:?} inside sign1 container {:?}",
            &protected,
            &sign1
        );
        let headers = sign1.unprotected.updated_with(protected);

        #[derive(minicbor::Encode)]
        struct SigStructureForSignature1<'a> {
            #[n(0)]
            context: &'static str,
            #[cbor(b(1), with = "minicbor::bytes")]
            body_protected: &'a [u8],
            #[cbor(b(2), with = "minicbor::bytes")]
            external_aad: &'a [u8],
            #[cbor(b(3), with = "minicbor::bytes")]
            payload: &'a [u8],
        }
        let aad = SigStructureForSignature1 {
            context: "Signature1",
            body_protected: sign1.protected,
            external_aad: &[],
            payload: sign1.payload,
        };
        buffer = heapless::Vec::new();
        minicbor::encode(&aad, minicbor_adapters::WriteToHeapless(&mut buffer))?;
        trace!("Serialized AAD: {:#02x}", buffer);

        let (scope, claims) = authorities.verify_asymmetric_token(
            &headers,
            &buffer,
            sign1.signature,
            sign1.payload,
            crate::PrivateMethod,
        )?;

        (scope, claims)
    } else {
        return Err(CredentialErrorDetail::UnsupportedExtension.into());
    };

    let time_constraint = crate::time::TimeConstraint::from_claims_set(&claims);

    let Cnf {
        osc: None,
        cose_key: Some(cose_key),
    } = claims.cnf
    else {
        return Err(CredentialErrorDetail::InconsistentDetails.into());
    };

    let mut prefixed = lakers::BufferCred::new();
    // The prefix for naked COSE_Keys from Section 3.5.2 of RFC9528
    prefixed
        .extend_from_slice(&[0xa1, 0x08, 0xa1, 0x01])
        .unwrap();
    prefixed
        .extend_from_slice(&cose_key.opaque)
        .map_err(|_| CredentialErrorDetail::ConstraintExceeded)?;
    let credential = lakers::Credential::new_ccs(
        prefixed,
        cose_key
            .parsed
            .x
            .ok_or(CredentialErrorDetail::InconsistentDetails)?
            .try_into()
            .map_err(|_| CredentialErrorDetail::InconsistentDetails)?,
    );

    Ok((credential, scope, time_constraint))
}
