//! Get atmospheric data from the US National Weather Service.
//!
//! Notes from https://www.weather.gov/documentation/services-web-api#:
//!
//! - /points/<lat>,<lon> returns:
//!     - .properties.observationStations, which is a URL to
//! /gridpoints/{wfo}/{x,y}/stations
//!     - ...and .properties.radarStation, which is a radar station ID
//!     - but *not* a stationID?
//! - The /gridpoints/{wfo}/{x,y}/stations returns:
//!     - .features[], a paginated list of observationStations, along
//!       with their coordinates; could be used to be more precise
//!     -   x,y here are not lat/lon, they are grid coordinates for the forecast
//! - /stations/{stationID}/observtions/latest returns:
//!     - .properties.temperature.{unitCode,value}
//!     - .properties.relativeHumidity.{unitCode,value}
//!     - no CO2 monitoring :(
//!
//! AirNow has an API [here](https://docs.airnowapi.org/)
//! that includes "lookup by lat/lon". Though, that's AQI, not CO2 PPM.
//!
//! US EPA has an [air quality API](https://aqs.epa.gov/aqsweb/documents/data_api.html#sample), but
//! it's historic only.
//! Google has an [Air Quality
//! API](https://developers.google.com/maps/documentation/air-quality/overview),
//! with a price of $5/1000q (!)
//!
