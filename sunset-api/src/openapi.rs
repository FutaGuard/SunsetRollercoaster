use utoipa::OpenApi;

use crate::error::ErrorBody;
use crate::handlers;
use crate::models::{
    FinanceExchangeRate, FireDispatchCase, FireDispatchCaseState, FireDispatchCounty,
    FireDispatchCountyInfo, FireDispatchRecord, FireDispatchStoredCase, Invoice, LiveExchangeRate,
    NationwideFuelPrice, Reservoir, TaipowerAreaLoad, TaipowerAreaSnapshot, TaipowerFuelMix,
    TaipowerGenerator, TaipowerOperatingReserve, TaipowerPowerSnapshot,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sunset Rollercoaster API",
        version = "0.1.0",
        description = include_str!("../docs/overview.md")
    ),
    paths(
        handlers::finance::list_exchange_rates,
        handlers::finance::latest_exchange_rates,
        handlers::fire_dispatch::list_counties,
        handlers::fire_dispatch::latest_all,
        handlers::fire_dispatch::latest_county,
        handlers::fire_dispatch::list_records,
        handlers::fire_dispatch::list_cases,
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
        FinanceExchangeRate,
        LiveExchangeRate,
        FireDispatchCase,
        FireDispatchCounty,
        FireDispatchCountyInfo,
        FireDispatchRecord,
        FireDispatchStoredCase,
        FireDispatchCaseState,
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
        (name = "finance", description = "Exchange rates from Google Finance; `rate` is the amount of `quote` (TWD) for one unit of `base`"),
        (name = "fire-dispatch", description = "Live cases from 18 county fire departments' public case lists, grouped by county, and their stored history; see the guide above"),
        (name = "fuel-prices", description = "Weekly Taiwan fuel and international crude oil prices"),
        (name = "invoices", description = "Taiwan uniform invoice winning numbers"),
        (name = "reservoirs", description = "Taiwan reservoir water level records"),
        (name = "taipower", description = "Taiwan Power Company load, generation, regional, and operating reserve data; all power values are normalized to MW")
    )
)]
pub struct ApiDoc;
