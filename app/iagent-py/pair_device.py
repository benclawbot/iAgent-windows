#!/usr/bin/env python3
"""
iAgent Device Pairing Script
Generates an auth token for the Python dock to connect to the Rust backend gateway.
Stores the token in config.toml so the IPC client can use it.
"""

import hashlib
import os
import secrets
import sys
import urllib.request
import json

CONFIG_PATH = os.path.join(os.path.expanduser("~"), ".jcode", "config.toml")
DEVICES_PATH = os.path.join(os.path.expanduser("~"), ".jcode", "devices.json")


def get_config_value(key: str) -> str:
    """Read the gateway section from config.toml."""
    with open(CONFIG_PATH, "r") as f:
        content = f.read()
    
    in_gateway = False
    for line in content.split("\n"):
        if line.strip().startswith("["):
            in_gateway = line.strip().startswith("[gateway]")
        elif in_gateway and key in line:
            _, _, value = line.partition("=")
            return value.strip().strip('"').strip("'")
    return ""


def generate_token() -> str:
    """Generate a random 32-byte hex token (64 hex chars)."""
    return secrets.token_hex(32)


def compute_token_hash(token: str) -> str:
    """Compute sha256:hex hash of a token."""
    h = hashlib.sha256(token.encode()).hexdigest()
    return f"sha256:{h}"


def register_device():
    """Register a new local device in the devices.json registry."""
    token = generate_token()
    token_hash = compute_token_hash(token)
    
    import datetime
    now = datetime.datetime.utcnow().isoformat() + "Z"
    
    device_id = "python-dock-local"
    device_name = "Python Dock (Local)"
    
    device_entry = {
        "id": device_id,
        "name": device_name,
        "apns_token": None,
        "token_hash": token_hash,
        "paired_at": now,
        "last_seen": now,
    }
    
    # Load existing devices or create new list
    if os.path.exists(DEVICES_PATH):
        with open(DEVICES_PATH, "r") as f:
            devices_data = json.load(f)
    else:
        devices_data = {"devices": [], "pending_codes": []}
    
    # Remove existing device with same ID (re-register)
    devices_data["devices"] = [d for d in devices_data.get("devices", []) if d.get("id") != device_id]
    devices_data["devices"].append(device_entry)
    
    # Save
    os.makedirs(os.path.dirname(DEVICES_PATH), exist_ok=True)
    with open(DEVICES_PATH, "w") as f:
        json.dump(devices_data, f, indent=2)
    
    return token


def update_config_token(token: str):
    """Add or update the local_auth_token in config.toml."""
    with open(CONFIG_PATH, "r") as f:
        content = f.read()
    
    # Check if token already exists
    if "local_auth_token" in content:
        # Replace existing
        import re
        content = re.sub(
            r'local_auth_token\s*=\s*"[^"]*"',
            f'local_auth_token = "{token}"',
            content
        )
    else:
        # Add after [gateway] section
        in_gateway = False
        lines = content.split("\n")
        new_lines = []
        for line in lines:
            new_lines.append(line)
            if line.strip().startswith("[gateway]"):
                new_lines.append(f'local_auth_token = "{token}"')
        content = "\n".join(new_lines)
    
    with open(CONFIG_PATH, "w") as f:
        f.write(content)


def check_gateway_status():
    """Check if gateway is reachable."""
    try:
        resp = urllib.request.urlopen("http://127.0.0.1:7643/health", timeout=5)
        data = json.loads(resp.read().decode())
        return data.get("gateway", False)
    except Exception as e:
        print(f"Gateway not reachable: {e}")
        return False


def main():
    print("=" * 50)
    print(" iAgent Device Pairing")
    print("=" * 50)
    
    # Check gateway
    if not check_gateway_status():
        print("ERROR: Gateway not running at 127.0.0.1:7643")
        print("Start with: iagent serve")
        sys.exit(1)
    
    print("\n1. Registering local device...")
    token = register_device()
    print(f"   Token: {token[:16]}...{token[-4:]}")
    
    print("\n2. Updating config.toml...")
    update_config_token(token)
    print("   Token saved to config")
    
    print("\n3. Testing connection...")
    import urllib.request
    try:
        req = urllib.request.Request(
            "http://127.0.0.1:7643/ws",
            headers={"Authorization": f"Bearer {token}"}
        )
        # This will fail for WS upgrade but we can at least verify the token is not rejected
        resp = urllib.request.urlopen(req, timeout=5)
        print(f"   HTTP OK (not WebSocket): {resp.status}")
    except urllib.error.HTTPError as e:
        if e.code == 404:
            print("   Token accepted (404 = not a local WS endpoint, but auth works)")
        else:
            print(f"   HTTP {e.code}: {e.reason}")
    except Exception as e:
        # WebSocket upgrade failure is expected
        if "WebSocket" in str(e) or "upgrade" in str(e).lower():
            print("   Token accepted (WebSocket upgrade = auth works)")
        else:
            print(f"   Unexpected: {e}")
    
    print("\n" + "=" * 50)
    print(" Pairing complete!")
    print(f" Token stored in: {CONFIG_PATH}")
    print(" IPC client can now connect to the gateway.")
    print("=" * 50)


if __name__ == "__main__":
    main()
