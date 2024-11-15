//! SigMF format.
//!
//! This module contains a minimal implementation of [SigMF](https://github.com/gnuradio/SigMF/).

use anyhow::Result;
use chrono::prelude::*;
use serde_json::json;

const SIGMF_VERSION: &str = "1.0.0";
const SIGMF_RECORDER: &str = concat!("Maia SDR v", env!("CARGO_PKG_VERSION"));

/// SigMF metadata.
///
/// This structure can be used to create and edit SigMF metadata, and convert it
/// to JSON format for its storage in a `.sigmf-meta` file.
///
/// # Examples
/// ```
/// use maia_httpd::sigmf::{Datatype, Field, Metadata, SampleFormat};
/// let datatype = Datatype { field: Field::Complex, format: SampleFormat::I8 };
/// let sample_rate = 1e6; // 1 Msps
/// let frequency = 100e6; // 100 MHz
/// let metadata = Metadata::new(datatype, sample_rate, frequency);
/// println!("{}", metadata.to_json());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    datatype: Datatype,
    sample_rate: f64,
    description: String,
    author: String,
    frequency: f64,
    datetime: DateTime<Utc>,
    geolocation: Option<GeoJsonPoint>,
}

/// SigMF datatype.
///
/// A datatype is formed by a field, which can be either real or complex, and a
/// sample format.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Datatype {
    /// Datatype field.
    ///
    /// This indicates if the signal is complex (IQ) or real.
    pub field: Field,
    /// Datatype sample format.
    ///
    /// The sample format indicates the width and format (floating point or
    /// integer) of the samples.
    pub format: SampleFormat,
}

impl std::fmt::Display for Datatype {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let field = match self.field {
            Field::Real => "r",
            Field::Complex => "c",
        };
        let (format, endianness) = match self.format {
            SampleFormat::F32(e) => ("f32", Some(e)),
            SampleFormat::F64(e) => ("f64", Some(e)),
            SampleFormat::I32(e) => ("i32", Some(e)),
            SampleFormat::I16(e) => ("i16", Some(e)),
            SampleFormat::U32(e) => ("u32", Some(e)),
            SampleFormat::U16(e) => ("u16", Some(e)),
            SampleFormat::I8 => ("i8", None),
            SampleFormat::U8 => ("u8", None),
        };
        let endianness = match endianness {
            Some(e) => match e {
                Endianness::Le => "_le",
                Endianness::Be => "_be",
            },
            None => "",
        };
        write!(f, "{field}{format}{endianness}")
    }
}

/// Datatype field.
///
/// A datatype [field](https://en.wikipedia.org/wiki/Field_(mathematics)) is used
/// to indicate if the signal is complex (IQ) or real.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Field {
    /// Real field.
    Real,
    /// Complex field.
    Complex,
}

/// Sample format.
///
/// The sample format indicates the width and type (floating point or integer)
/// of the numbers used to represent the signal samples.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SampleFormat {
    /// 32-bit IEEE 754 floating point.
    F32(Endianness),
    /// 64-bit IEEE 754 floating point.
    F64(Endianness),
    /// 32-bit signed integer.
    I32(Endianness),
    /// 16-bit signed integer.
    I16(Endianness),
    /// 32-bit unsigned integer.
    U32(Endianness),
    /// 16-bit unsigned integer.
    U16(Endianness),
    /// 8-bit signed integer.
    I8,
    /// 8-bit unsigned integer.
    U8,
}

/// Endianness.
///
/// The endianness indicates the order of the bytes forming a multi-byte number
/// in memory or in a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Endianness {
    /// Little-endian.
    Le,
    /// Big-endian.
    Be,
}

impl From<maia_json::RecorderMode> for Datatype {
    fn from(value: maia_json::RecorderMode) -> Datatype {
        match value {
            maia_json::RecorderMode::IQ8bit => Datatype {
                field: Field::Complex,
                format: SampleFormat::I8,
            },
            maia_json::RecorderMode::IQ12bit | maia_json::RecorderMode::IQ16bit => Datatype {
                field: Field::Complex,
                format: SampleFormat::I16(Endianness::Le),
            },
        }
    }
}

/// GeoJSON point.
///
/// This struct represents a GeoJSON point, which contains a latitude and
/// longitude with respect to the WGS84 ellipsoid, and an optional altitude.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GeoJsonPoint {
    latitude: f64,
    longitude: f64,
    altitude: Option<f64>,
}

impl TryFrom<maia_json::Geolocation> for GeoJsonPoint {
    type Error = anyhow::Error;

    fn try_from(value: maia_json::Geolocation) -> Result<GeoJsonPoint> {
        GeoJsonPoint::from_lat_lon_alt_option(value.latitude, value.longitude, value.altitude)
    }
}

impl From<GeoJsonPoint> for maia_json::Geolocation {
    fn from(value: GeoJsonPoint) -> maia_json::Geolocation {
        maia_json::Geolocation {
            altitude: value.altitude,
            latitude: value.latitude,
            longitude: value.longitude,
        }
    }
}

impl GeoJsonPoint {
    /// Creates a GeoJSON point from a latitude and longitude.
    ///
    /// The latitude is given in degrees, between -90 and 90. The longitude is
    /// given in degrees, between -180 and 180. An error is returned if the
    /// values are out of range.
    pub fn from_lat_lon(latitude: f64, longitude: f64) -> Result<GeoJsonPoint> {
        GeoJsonPoint::from_lat_lon_alt_option(latitude, longitude, None)
    }

    /// Creates a GeoJSON point from a latitude, longitude and altitude.
    ///
    /// The latitude is given in degrees, between -90 and 90. The longitude is
    /// given in degrees, between -180 and 180. The altitude is given in
    /// meters. An error is returned if the values are out of range.
    pub fn from_lat_lon_alt(latitude: f64, longitude: f64, altitude: f64) -> Result<GeoJsonPoint> {
        GeoJsonPoint::from_lat_lon_alt_option(latitude, longitude, Some(altitude))
    }

    /// Creates a GeoJSON point from a latitude, longitude and an optional
    /// altitude.
    ///
    /// The latitude is given in degrees, between -90 and 90. The longitude is
    /// given in degrees, between -180 and 180. The altitude is given in
    /// meters. An error is returned if the values are out of range.
    pub fn from_lat_lon_alt_option(
        latitude: f64,
        longitude: f64,
        altitude: Option<f64>,
    ) -> Result<GeoJsonPoint> {
        anyhow::ensure!(
            (-90.0..=90.0).contains(&latitude),
            "latitude is not between -90 and +90 degrees"
        );
        anyhow::ensure!(
            (-180.0..=180.0).contains(&longitude),
            "longitude is not between -180 and +180 degrees"
        );
        Ok(GeoJsonPoint {
            latitude,
            longitude,
            altitude,
        })
    }

    /// Gives the latitude of the GeoJSON point in degrees.
    pub fn latitude(&self) -> f64 {
        self.latitude
    }

    /// Gives the longitude of the GeoJSON point in degrees.
    pub fn longitude(&self) -> f64 {
        self.longitude
    }

    /// Gives the altitude of the GeoJSON point.
    ///
    /// The altitude is returned in meters, or `None` if the point does not
    /// contain an altitude.
    pub fn altitude(&self) -> Option<f64> {
        self.altitude
    }

    /// Returns a JSON [`serde_json::Value`] that represents the GeoJSON point in JSON.
    ///
    /// The formatting of the JSON is compliant with the SigMF standard.
    pub fn to_json_value(&self) -> serde_json::Value {
        if let Some(altitude) = self.altitude {
            json!({
                "type": "Point",
                "coordinates": [self.longitude, self.latitude, altitude]
            })
        } else {
            json!({
                "type": "Point",
                "coordinates": [self.longitude, self.latitude]
            })
        }
    }
}

impl Metadata {
    /// Creates a new SigMF metadata object.
    ///
    /// The datatype, sample rate and frequency are mandatory parameters. The
    /// datetime field is set to the current time. The description and author
    /// fields are initialized to empty strings.
    pub fn new(datatype: Datatype, sample_rate: f64, frequency: f64) -> Metadata {
        Metadata {
            datatype,
            sample_rate,
            description: String::new(),
            author: String::new(),
            frequency,
            datetime: Utc::now(),
            geolocation: None,
        }
    }

    /// Gives the value of the datatype field.
    pub fn datatype(&self) -> Datatype {
        self.datatype
    }

    /// Sets the value datatype field.
    pub fn set_datatype(&mut self, datatype: Datatype) {
        self.datatype = datatype;
    }

    /// Gives the value of the sample rate field (in samples per second).
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Sets the value of the sample rate field.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    /// Gives the value of the description field.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Sets the value of the description field.
    pub fn set_description(&mut self, description: &str) {
        self.description.replace_range(.., description);
    }

    /// Gives the value of the author field.
    pub fn author(&self) -> &str {
        &self.author
    }

    /// Sets the value of the author field.
    pub fn set_author(&mut self, author: &str) {
        self.author.replace_range(.., author);
    }

    /// Gives the value of the frequency field (in Hz).
    pub fn frequency(&self) -> f64 {
        self.frequency
    }

    /// Gives the value of the geolocation field.
    pub fn geolocation(&self) -> Option<GeoJsonPoint> {
        self.geolocation
    }

    /// Sets the value of the frequency field.
    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
    }

    /// Gives the value of the datetime field.
    pub fn datetime(&self) -> DateTime<Utc> {
        self.datetime
    }

    /// Sets the value of the datetime field.
    pub fn set_datetime(&mut self, datetime: DateTime<Utc>) {
        self.datetime = datetime;
    }

    /// Sets the datetime field to the current time.
    pub fn set_datetime_now(&mut self) {
        self.set_datetime(Utc::now());
    }

    /// Sets the value of the geolocation field.
    pub fn set_geolocation(&mut self, geolocation: GeoJsonPoint) {
        self.geolocation = Some(geolocation);
    }

    /// Removes the geolocation field.
    pub fn remove_geolocation(&mut self) {
        self.geolocation = None;
    }

    /// Sets or removes the value of the geolocation field.
    ///
    /// If `geolocation` is `Some`, then the value of the geolocation field is
    /// set. Otherwise, the value is cleared.
    pub fn set_geolocation_optional(&mut self, geolocation: Option<GeoJsonPoint>) {
        self.geolocation = geolocation;
    }

    /// Returns a string that represents the metadata in JSON.
    ///
    /// The formatting of the JSON is compliant with the SigMF standard.
    pub fn to_json(&self) -> String {
        let json = self.to_json_value();
        let mut s = serde_json::to_string_pretty(&json).unwrap();
        s.push('\n'); // to_string_pretty does not include a final \n
        s
    }

    /// Returns a JSON [`serde_json::Value`] that represents the metadata in JSON.
    ///
    /// The formatting of the JSON is compliant with the SigMF standard.
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut global = json!({
            "core:datatype": self.datatype.to_string(),
            "core:version": SIGMF_VERSION,
            "core:sample_rate": self.sample_rate,
            "core:description": self.description,
            "core:author": self.author,
            "core:recorder": SIGMF_RECORDER
        });
        if let Some(geolocation) = self.geolocation() {
            global
                .as_object_mut()
                .unwrap()
                .insert("core:geolocation".to_string(), geolocation.to_json_value());
        }
        json!({
            "global": global,
            "captures": [
                {
                    "core:sample_start": 0,
                    "core:frequency": self.frequency,
                    "core:datetime": self.datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
                }
            ],
            "annotations": []
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn to_json() {
        let meta = Metadata {
            datatype: Datatype {
                field: Field::Complex,
                format: SampleFormat::I16(Endianness::Le),
            },
            sample_rate: 30.72e6,
            description: "Test SigMF dataset".to_string(),
            author: "Tester".to_string(),
            frequency: 2400e6,
            datetime: Utc.with_ymd_and_hms(2022, 11, 1, 0, 0, 0).unwrap(),
            geolocation: None,
        };
        let json = meta.to_json();
        let expected = [
            r#"{
  "annotations": [],
  "captures": [
    {
      "core:datetime": "2022-11-01T00:00:00.000Z",
      "core:frequency": 2400000000.0,
      "core:sample_start": 0
    }
  ],
  "global": {
    "core:author": "Tester",
    "core:datatype": "ci16_le",
    "core:description": "Test SigMF dataset",
    "core:recorder": ""#,
            SIGMF_RECORDER,
            r#"",
    "core:sample_rate": 30720000.0,
    "core:version": ""#,
            SIGMF_VERSION,
            r#""
  }
}
"#,
        ]
        .join("");
        assert_eq!(json, expected);
    }

    #[test]
    fn to_json_with_geolocation() {
        let meta = Metadata {
            datatype: Datatype {
                field: Field::Complex,
                format: SampleFormat::I16(Endianness::Le),
            },
            sample_rate: 30.72e6,
            description: "Test SigMF dataset with geolocation".to_string(),
            author: "Tester".to_string(),
            frequency: 2400e6,
            datetime: Utc.with_ymd_and_hms(2022, 11, 1, 0, 0, 0).unwrap(),
            geolocation: Some(
                GeoJsonPoint::from_lat_lon_alt(34.0787916, -107.6183682, 2120.0).unwrap(),
            ),
        };
        let json = meta.to_json();
        let expected = [
            r#"{
  "annotations": [],
  "captures": [
    {
      "core:datetime": "2022-11-01T00:00:00.000Z",
      "core:frequency": 2400000000.0,
      "core:sample_start": 0
    }
  ],
  "global": {
    "core:author": "Tester",
    "core:datatype": "ci16_le",
    "core:description": "Test SigMF dataset with geolocation",
    "core:geolocation": {
      "coordinates": [
        -107.6183682,
        34.0787916,
        2120.0
      ],
      "type": "Point"
    },
    "core:recorder": ""#,
            SIGMF_RECORDER,
            r#"",
    "core:sample_rate": 30720000.0,
    "core:version": ""#,
            SIGMF_VERSION,
            r#""
  }
}
"#,
        ]
        .join("");
        assert_eq!(json, expected);
    }
}
