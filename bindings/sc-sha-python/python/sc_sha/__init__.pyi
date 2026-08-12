from typing import Mapping, Any

class ScShaError(Exception):
    code: str
    message: str

def calculate_hash(input: Mapping[str, Any]) -> dict[str, str]: ...
def calculate_composition_hash(manifest: Mapping[str, Any]) -> dict[str, str]: ...
