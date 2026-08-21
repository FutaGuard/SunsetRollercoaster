use axum::{extract::Query, http::Uri};
use sunset_api::handlers::{
    fuel_price::FuelPriceListParams,
    invoice::InvoiceListParams,
    reservoir::ReservoirListParams,
    taipower::{GeneratorListParams, ReserveListParams, TimeListParams},
};
use sunset_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn uri(path: &str) -> Uri {
    format!("{path}?start=2026-08-14&end=2026-08-21")
        .parse()
        .unwrap()
}

fn text<T: ToString>(value: Option<&T>) -> Option<String> {
    value.map(ToString::to_string)
}

#[test]
fn taipower_time_list_endpoints_accept_date_only_ranges() {
    let Query(params) = Query::<TimeListParams>::try_from_uri(&uri("/taipower/fuel-mix"))
        .expect("date-only start and end should be valid");

    assert_eq!(text(params.start.as_ref()), Some("2026-08-14".to_string()));
    assert_eq!(text(params.end.as_ref()), Some("2026-08-21".to_string()));
}

#[test]
fn taipower_generator_endpoint_accepts_date_only_ranges() {
    let Query(params) = Query::<GeneratorListParams>::try_from_uri(&uri("/taipower/generators"))
        .expect("date-only start and end should be valid");

    assert_eq!(text(params.start.as_ref()), Some("2026-08-14".to_string()));
    assert_eq!(text(params.end.as_ref()), Some("2026-08-21".to_string()));
}

#[test]
fn reservoir_endpoint_accepts_date_only_ranges() {
    let Query(params) = Query::<ReservoirListParams>::try_from_uri(&uri("/reservoirs"))
        .expect("date-only start and end should be valid");

    assert_eq!(text(params.start.as_ref()), Some("2026-08-14".to_string()));
    assert_eq!(text(params.end.as_ref()), Some("2026-08-21".to_string()));
}

#[test]
fn existing_date_range_endpoints_keep_accepting_date_only_ranges() {
    let Query(fuel_prices) =
        Query::<FuelPriceListParams>::try_from_uri(&uri("/fuel-prices")).unwrap();
    let Query(invoices) = Query::<InvoiceListParams>::try_from_uri(&uri("/invoices")).unwrap();
    let Query(reserves) =
        Query::<ReserveListParams>::try_from_uri(&uri("/taipower/operating-reserves")).unwrap();

    assert_eq!(
        (
            text(fuel_prices.start.as_ref()),
            text(fuel_prices.end.as_ref())
        ),
        (
            Some("2026-08-14".to_string()),
            Some("2026-08-21".to_string())
        )
    );
    assert_eq!(
        (text(invoices.start.as_ref()), text(invoices.end.as_ref())),
        (
            Some("2026-08-14".to_string()),
            Some("2026-08-21".to_string())
        )
    );
    assert_eq!(
        (text(reserves.start.as_ref()), text(reserves.end.as_ref())),
        (
            Some("2026-08-14".to_string()),
            Some("2026-08-21".to_string())
        )
    );
}

#[test]
fn openapi_describes_every_date_range_as_date_only() {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let paths = document["paths"].as_object().unwrap();
    let mut checked = Vec::new();

    for (path, item) in paths {
        let Some(parameters) = item["get"]["parameters"].as_array() else {
            continue;
        };

        for parameter in parameters {
            let name = parameter["name"].as_str().unwrap();
            if matches!(name, "start" | "end") {
                assert_eq!(
                    parameter["schema"]["format"].as_str(),
                    Some("date"),
                    "{path} `{name}` must be date-only"
                );
                checked.push((path, name));
            }
        }
    }

    assert_eq!(
        checked.len(),
        18,
        "expected start/end on nine list endpoints"
    );
}
