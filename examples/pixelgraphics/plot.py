import asyncio
import sys

import aiocoap
import aiocoap.edhoc
import cbor2

remote = sys.argv[1]
match sys.argv[2]:
    case "colors":
        path = "fb"
        method = aiocoap.PUT

        columns = int(sys.argv[3])
        rows = int(sys.argv[4])
        pixels = [[(x / columns) ** 1.8, (1 - x / columns) ** 2.2, (y / rows) ** 3.3] for y in range(rows) for x in range(columns)]

        encoded = cbor2.CBORTag(40, [[rows, columns, 3], cbor2.CBORTag(64, bytes(int(4 * channel) for pixel in pixels for channel in pixel))])
        encoded = cbor2.dumps(encoded)
    case "stop":
        path = "text"
        method = aiocoap.DELETE
        encoded = b""
    case text:
        path = "text"
        method = aiocoap.PUT
        encoded = cbor2.dumps(text)

async def send():
    ctx = await aiocoap.Context.create_client_context()
    # The demo keys (as we don't have storage yet so we can't use real ones)
    ctx.client_credentials["*"] = aiocoap.edhoc.EdhocCredentials(
            suite=2,
            method=3,
            own_cred_style="by-key-id",
            own_cred={14: {2: "42-50-31-FF-EF-37-32-39", 8: {1: {1: 2, 2: bytes.fromhex('2b'), -1: 1, -2: bytes.fromhex('ac75e9ece3e50bfc8ed60399889522405c47bf16df96660a41298cb4307f7eb6'), -3: bytes.fromhex('6e5de611388a4b8a8211334ac7d37ecb52a387d257e6db3c2a93df21ff3affc8')}}}},
            private_key={1: 2, -1: 1, -4: bytes.fromhex('fb13adeb6518cee5f88417660841142e830a81fe334380a953406a1305e8706b')},
            peer_cred={14: {2: "", 8: {1: {1: 2, 2: bytes.fromhex('0a'), -1: 1, -2: bytes.fromhex('bbc34960526ea4d32e940cad2a234148ddc21791a12afbcbac93622046dd44f0'), -3: bytes.fromhex('4519e257236b2a0ce2023f0931f1f386ca7afda64fcde0108c224c51eabf6072')}}}},
            )
    await ctx.request(aiocoap.Message(code=method, uri=remote, uri_path=[path], payload=encoded)).response_raising

asyncio.run(send())
