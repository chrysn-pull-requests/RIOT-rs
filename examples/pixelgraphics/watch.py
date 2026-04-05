"""Shows a window that is constantly being updated with polled framebuffer
state received over the network"""

import asyncio
import itertools
import sys
import threading

import aiocoap
import cbor2
import cbor_diag

import numpy as np
from matplotlib import pyplot as plt

uri = sys.argv[1]

fig = plt.figure()
ax = fig.add_subplot(1, 1, 1)
ax.set_title(f"Waiting for {uri}")
# we probably need to set some axes staight later, but this way we can separate setup from the rest
theplot = ax.imshow([[1,2],[3,4]], interpolation='nearest')
fig.show()

async def watch():
    ctx = await aiocoap.Context.create_client_context()
    ctx.client_credentials.load_from_dict(cbor2.loads(cbor_diag.diag2cbor(open("client-credentials.diag").read())))

    # FIXME: aiocoap should take care of this, to not run edhoc all the time, and to reconnect once the context is broken
    remote = None

    while True:
        request = aiocoap.Message(code=aiocoap.GET, uri=uri)
        if remote is not None:
            response.remote = remote
        response = await ctx.request(request).response_raising
        remote = response.remote

        parsed = cbor2.loads(response.payload)
        # some asserts are implied by field access; this is software that can
        # just crash when the peer does something unexpected.
        assert parsed.tag == 40
        [dimensions, pixels] = parsed.value
        match dimensions:
            case [y, x, 3]:
                pass
            case [y, x, 1] | [x, y]:
                raise UnimplementedError("Monochrome support are not yet implemented")
            case _:
                raise ValueError("I don't understand how that should be an image")
        assert pixels.tag == 64, "Currenlty, only 8 bit per pixel are supported"

        # apply some "gain control": sometimes we set values very dim b/c some
        # displays are just too bright.

        # but at least 10: if it's very very dark, keep it dark but visible
        maximum = max(max(pixels.value), 10)
        # floor division: if any value is > 128, we don't exaggerate
        gain = 255 // maximum

        # there's likely an np constructor that'd do that for us instead, but I don't know it
        rgb = itertools.batched([chan * gain for chan in pixels.value], 3)
        rows = itertools.batched(rgb, x)
        pixels = np.array(list(rows), dtype='u8')

        ax.set_title(f"Frame buffer content of {uri}: {x} x {y} (brightened by factor {gain})")
        theplot.set_data(pixels)
        theplot.set_extent([-0.5, x - 0.5, y - 0.5, -0.5])

        # going for 0.1 eventually
        await asyncio.sleep(2)

# matplotlib insists it gets the main thread. async doesn't care, so we run
# aiocoap off-main-thread. didn't try that before, so good testing :-D
threading.Thread(target=asyncio.run, args=(watch(),)).start()

plt.ion()
fig.canvas.start_event_loop()
