//! Deliberately invalid fixture: sc-sha must not depend on sc-composer.

pub fn calculate_hash() -> sc_composer::Renderer {
    sc_composer::Renderer
}
