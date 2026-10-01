"""LOD Manager REST API Blueprint for Quart / Telegram Bot Backend."""

from quart import Blueprint

api_bp = Blueprint("api_v1", __name__, url_prefix="/api/v1")

# Import routes to register endpoints on blueprint
from . import routes  # noqa: E402, F401

__all__ = ["api_bp"]
