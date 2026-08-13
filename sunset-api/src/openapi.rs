use utoipa::OpenApi;

use crate::error::ErrorBody;
use crate::handlers;
use crate::models::{
    Invoice, NationwideFuelPrice, Reservoir, TaipowerAreaLoad, TaipowerAreaSnapshot,
    TaipowerFuelMix, TaipowerGenerator, TaipowerOperatingReserve, TaipowerPowerSnapshot,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sunset Rollercoaster API",
        version = "0.1.0",
        description = "Read-only REST API for Taipower electricity data, Taiwan and international fuel prices, invoice draws, and reservoir records."
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
        handlers::taipower::list_power_snapshots,
        handlers::taipower::latest_power_snapshot,
        handlers::taipower::list_fuel_mix,
        handlers::taipower::latest_fuel_mix,
        handlers::taipower::list_area_loads,
        handlers::taipower::latest_area_load,
        handlers::taipower::list_area_snapshots,
        handlers::taipower::latest_area_snapshot,
        handlers::taipower::list_operating_reserves,
        handlers::taipower::latest_operating_reserve,
        handlers::taipower::list_generators,
        handlers::taipower::latest_generators,
    ),
    components(schemas(
        NationwideFuelPrice,
        Invoice,
        Reservoir,
        TaipowerPowerSnapshot,
        TaipowerFuelMix,
        TaipowerAreaLoad,
        TaipowerAreaSnapshot,
        TaipowerOperatingReserve,
        TaipowerGenerator,
        ErrorBody
    )),
    tags(
        (name = "fuel-prices", description = "Weekly Taiwan fuel and international crude oil prices"),
        (name = "invoices", description = "Taiwan uniform invoice winning numbers"),
        (name = "reservoirs", description = "Taiwan reservoir water level records"),
        (name = "taipower", description = "Taiwan Power Company load, generation, regional, and operating reserve data; all power values are normalized to MW")
    )
)]
pub struct ApiDoc;
