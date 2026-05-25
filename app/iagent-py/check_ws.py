"""Check websockets version and fix API compatibility."""
import websockets
print(f"websockets version: {websockets.__version__}")

# Check if extra_headers is supported
import inspect
try:
    sig = inspect.signature(websockets.connect)
    print(f"connect signature: {sig}")
except:
    pass

# Check client factory
from websockets.asyncio.client import connect as aconnect
try:
    sig = inspect.signature(aconnect)
    print(f"asyncio connect signature: {sig}")
except:
    pass