use utoipa::OpenApi;

use crate::error::ErrorBody;
use crate::handlers;
use crate::models::{Invoice, Reservoir};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sunset Rollercoaster API",
        version = "0.1.0",
        description = "Read-only REST API for invoice draws and Taiwan reservoir records."
    ),
    paths(
        handlers::invoice::list_invoices,
        handlers::invoice::latest_invoice,
        handlers::invoice::get_invoice,
        handlers::reservoir::list_reservoirs,
        handlers::reservoir::list_reservoir_names,
        handlers::reservoir::latest_reservoirs,
        handlers::reservoir::get_reservoir,
    ),
    components(schemas(Invoice, Reservoir, ErrorBody)),
    tags(
        (name = "invoices", description = "Taiwan uniform invoice winning numbers"),
        (name = "reservoirs", description = "Taiwan reservoir water level records")
    )
)]
pub struct ApiDoc;
