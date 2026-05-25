import unittest
from unittest.mock import patch

import pair_device


class PairDeviceTests(unittest.TestCase):
    def test_generate_token_uses_cryptographic_hex_token(self):
        with patch("pair_device.secrets.token_hex", return_value="a" * 64) as token_hex:
            self.assertEqual(pair_device.generate_token(), "a" * 64)
            token_hex.assert_called_once_with(32)


if __name__ == "__main__":
    unittest.main()
