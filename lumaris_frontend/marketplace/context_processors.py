"""this file to expose ENV to the template context"""

from django.conf import settings


def lumaris_settings(_request):
    """Expose LUMARIS_WS_URL to the template context."""
    return {"LUMARIS_WS_URL": settings.LUMARIS_WS_URL}
