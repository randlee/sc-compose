mod json;
mod logger;
#[cfg(test)]
mod tests;
mod text;

use sc_observability_types::ValueValidationError;

pub(crate) const SERVICE_NAME: &str = "sc-compose";

pub(crate) fn validated_service_name() -> Result<sc_observability::ServiceName, ValueValidationError>
{
    sc_observability::ServiceName::new(SERVICE_NAME)
}

pub(crate) use json::health_json_value;
pub(crate) use logger::build_logger;
#[cfg(test)]
pub(crate) use logger::build_logger_for_root;
pub(crate) use text::print_observability_health;
