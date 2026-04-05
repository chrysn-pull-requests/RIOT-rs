import asyncio
import sys

import aiocoap
import cbor2
import cbor_diag

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
    case "lava":
        path = "ll"
        method = aiocoap.POST
        encoded = b""
    case text:
        path = "text"
        method = aiocoap.PUT
        encoded = cbor2.dumps(text)

async def send():
    ctx = await aiocoap.Context.create_client_context()
    # The demo keys (as we don't have storage yet so we can't use real ones)
    ctx.client_credentials.load_from_dict(cbor2.loads(cbor_diag.diag2cbor(open("client-credentials.diag").read())))
    await ctx.request(aiocoap.Message(code=method, uri=remote, uri_path=[path], payload=encoded)).response_raising

asyncio.run(send())
