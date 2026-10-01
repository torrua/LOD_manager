"""Telegram WebApp authentication and HMAC-SHA256 verification."""

import hashlib
import hmac
import json
import os
import time
import urllib.parse
from functools import wraps
from typing import Any, Dict, Optional, Set, Tuple

from quart import current_app, g, jsonify, request


def get_admin_ids() -> Set[int]:
    """Retrieve allowed admin Telegram user IDs from config or environment."""
    config_admins = current_app.config.get("ADMIN_IDS")
    if isinstance(config_admins, (set, list, tuple)):
        return {int(x) for x in config_admins}

    env_admins = os.environ.get("ADMIN_IDS", "")
    admins: Set[int] = set()
    for part in env_admins.split(","):
        part = part.strip()
        if part.isdigit():
            admins.add(int(part))
    return admins


def get_bot_token() -> str:
    """Retrieve Telegram Bot Token from config or environment."""
    return (
        current_app.config.get("BOT_TOKEN")
        or os.environ.get("BOT_TOKEN")
        or ""
    )


def validate_init_data(
    init_data_raw: str,
    bot_token: str,
    ttl_seconds: int = 86400,
) -> Tuple[bool, Optional[Dict[str, Any]], Optional[str]]:
    """
    Validates Telegram WebApp initData string using HMAC-SHA256 according to Telegram specifications.
    
    Telegram Algorithm:
    1. Parse raw query string.
    2. Extract 'hash' parameter.
    3. Sort remaining parameters alphabetically in 'key=value\\n' format.
    4. Compute secret_key = HMAC_SHA256(key="WebAppData", msg=bot_token).
    5. Compute calculated_hash = HMAC_SHA256(key=secret_key, msg=data_check_string).
    6. Verify hash equality with hmac.compare_digest.
    7. Validate auth_date against TTL.
    """
    if not init_data_raw:
        return False, None, "Missing initData"

    if not bot_token:
        return False, None, "Server missing BOT_TOKEN"

    try:
        parsed = dict(urllib.parse.parse_qsl(init_data_raw, keep_blank_values=True))
    except Exception as exc:
        return False, None, f"Failed to parse initData: {exc}"

    received_hash = parsed.pop("hash", None)
    if not received_hash:
        return False, None, "Missing hash in initData"

    auth_date_str = parsed.get("auth_date")
    if not auth_date_str:
        return False, None, "Missing auth_date in initData"

    try:
        auth_date = int(auth_date_str)
        if time.time() - auth_date > ttl_seconds:
            return False, None, "initData has expired"
    except ValueError:
        return False, None, "Invalid auth_date integer"

    # Sort remaining key-value pairs alphabetically
    data_check_string = "\n".join(f"{k}={v}" for k, v in sorted(parsed.items()))

    # secret_key = HMAC_SHA256(b"WebAppData", bot_token)
    secret_key = hmac.new(b"WebAppData", bot_token.encode("utf-8"), hashlib.sha256).digest()

    # calculated_hash = HMAC_SHA256(secret_key, data_check_string).hexdigest()
    calculated_hash = hmac.new(
        secret_key, data_check_string.encode("utf-8"), hashlib.sha256
    ).hexdigest()

    if not hmac.compare_digest(calculated_hash, received_hash):
        return False, None, "Hash mismatch"

    user_data: Optional[Dict[str, Any]] = None
    if "user" in parsed:
        try:
            user_data = json.loads(parsed["user"])
        except json.JSONDecodeError:
            pass

    return True, user_data, None


def get_auth_token_from_request() -> Optional[str]:
    """Extract raw Telegram initData string from request headers or query params."""
    # Preferred custom header
    header = request.headers.get("X-Telegram-Init-Data")
    if header:
        return header

    # Standard Authorization header: Authorization: tma <initData>
    auth = request.headers.get("Authorization")
    if auth and auth.startswith("tma "):
        return auth[4:].strip()

    # Query param fallback for debugging / direct links
    param = request.args.get("init_data")
    if param:
        return param

    return None


def require_admin(f):
    """Decorator to require authenticated Telegram user with administrator privileges."""
    @wraps(f)
    async def decorated(*args, **kwargs):
        init_data = get_auth_token_from_request()
        if not init_data:
            return jsonify({"error": "Unauthorized: missing Telegram initData"}), 401

        bot_token = get_bot_token()
        valid, user, err = validate_init_data(init_data, bot_token)
        if not valid or not user:
            return jsonify({"error": f"Unauthorized: {err or 'Invalid auth'}"}), 401

        admin_ids = get_admin_ids()
        user_id = user.get("id")
        if not user_id or user_id not in admin_ids:
            return jsonify({"error": "Forbidden: Admin privileges required"}), 403

        g.tg_user = user
        g.is_admin = True
        return await f(*args, **kwargs)

    return decorated


def optional_auth(f):
    """Decorator to optionally parse Telegram user info if provided without rejecting guest reads."""
    @wraps(f)
    async def decorated(*args, **kwargs):
        g.tg_user = None
        g.is_admin = False

        init_data = get_auth_token_from_request()
        if init_data:
            bot_token = get_bot_token()
            valid, user, _ = validate_init_data(init_data, bot_token)
            if valid and user:
                g.tg_user = user
                admin_ids = get_admin_ids()
                g.is_admin = user.get("id") in admin_ids

        return await f(*args, **kwargs)

    return decorated
