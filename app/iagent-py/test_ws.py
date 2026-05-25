"""Test WebSocket connection to iAgent gateway."""
import asyncio
import websockets
import sys

async def test():
    try:
        print('Attempting WebSocket connection to ws://127.0.0.1:7643...')
        ws = await asyncio.wait_for(
            websockets.connect('ws://127.0.0.1:7643', ping_interval=None),
            timeout=5.0
        )
        print('Connected!')
        # Try sending a ping
        await ws.send('{"type":"ping","id":1}')
        msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
        print('Received:', msg)
        await ws.close()
    except Exception as e:
        print(f'Error: {type(e).__name__}: {e}')
        sys.exit(1)

asyncio.run(test())