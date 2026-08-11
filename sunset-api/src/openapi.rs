use utoipa::OpenApi;

use crate::error::ErrorBody;
use crate::handlers;
use crate::models::{Invoice, NationwideFuelPrice, Reservoir};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sunset Rollercoaster API",
        version = "0.1.0",
        description = "Read-only REST API for Taiwan fuel prices, invoice draws, and reservoir records."
    ),
    paths(
        handlers::fuel_price::list_fuel_prices,
        handlers::fuel_price::latest_fuel_price,
        handlers::fuel_price::get_fuel_price,
        handlers::invoice::list_invoices,
        handlers::invoice::latest_invoice,
        handlers::invoice::get_invoice,
        handlers::reservoir::list_reservoirs,
        handlers::reservoir::list_reservoir_names,
        handlers::reservoir::latest_reservoirs,
        handlers::reservoir::get_reservoir,
    ),
    components(schemas(NationwideFuelPrice, Invoice, Reservoir, ErrorBody)),
    tags(
        (name = "fuel-prices", description = "Taiwan nationwide weekly fuel prices"),
        (name = "invoices", description = "Taiwan uniform invoice winning numbers"),
        (name = "reservoirs", description = "Taiwan reservoir water level records")
    )
)]
pub struct ApiDoc;
