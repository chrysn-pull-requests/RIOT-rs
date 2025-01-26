//! Traits and types around representing time, as used to consider token expiration.

/// A clock by which time stamps on authorization credentials are compared.
///
/// It is yet unspecified whether timestamps are given in Unix time (UTC) or TAI.
///
/// # Evolution
///
/// Currently this is set up to provide ray time. It may need more interfaces later in order to
/// also accommodate usages where a `cnonce` is generated (which may then be used to either just
/// validate a token's time constraints, or may be used together with an `iat` in the subsequent
/// token to enhance the device's understanding of time).
///
/// Given that the 2038 problem can be mitigated also by using a different offset, we might
/// consider switching to an internal u32 based type that expresses Unix time / TAI with an offset
/// -- while the 130 years expressible with it are coming to an end, I'm relatively sure that we
/// can get away with limiting the usable range to 130 years starting from when a concrete firmware
/// is built.
pub trait TimeProvider {
    /// Confidence interval of the clock as lower and upper bound.
    fn now(&mut self) -> (u64, Option<u64>);

    /// Informs the clock that a credential has been ingested from a trusted AS that claims this
    /// time to be in the past.
    #[expect(
        unused_variables,
        reason = "Names are human visible part of API description"
    )]
    fn past_trusted(&mut self, timestamp: u64) {}
}

impl<T: TimeProvider> TimeProvider for &mut T {
    fn now(&mut self) -> (u64, Option<u64>) {
        (*self).now()
    }

    fn past_trusted(&mut self, timestamp: u64) {
        (*self).past_trusted(timestamp)
    }
}

/// A time provider that knows nothing.
///
/// It ignores any input, and always produces the maximum uncertainty.
pub struct TimeUnknown;

impl TimeProvider for TimeUnknown {
    #[inline]
    fn now(&mut self) -> (u64, Option<u64>) {
        (0, None)
    }
}

/// A processed set of token claims that limit it in time.
pub struct TimeConstraint {
    // iat would not go in here (that's only to feed a `TimeProvider::past_trusted`; nbf would go
    // in here but we don't read that yet)
    exp: Option<u64>,
}

impl TimeConstraint {
    /// Creates a [`TimeConstraints`] with no bounds; it is valid at any time.
    pub(crate) fn unbounded() -> Self {
        Self { exp: None }
    }

    /// Extrat time constraint from a claim.
    ///
    /// This is infallible as long as all relevant constraints on the value can be encoded in the
    /// ace module; doing that is preferable because it eases error tracking.
    pub fn from_claims_set(claims: &crate::ace::CwtClaimsSet<'_>) -> Self {
        TimeConstraint {
            exp: Some(claims.exp),
        }
    }

    /// Evaluates the constraint against time provided by the time provider.
    ///
    /// Any uncertainty of the time provider is counted for the benefit of the client.
    pub(crate) fn is_valid_with(&self, time_provider: &mut impl TimeProvider) -> bool {
        let Some(exp) = self.exp else {
            return true;
        };
        let (now_early, _now_late) = time_provider.now();

        exp > now_early
    }
}
