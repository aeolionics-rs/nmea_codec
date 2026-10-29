//  SPDX-FileCopyrightText: 2026-2026. Aeolionics, LLC
//
//  SPDX-License-Identifier: Apache-2.0

//! Common data types used within messages.
//!
use std::fmt::{Display, Formatter, Write};
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};
use uom::si::velocity::knot;

/// An identifier for a device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Talker {
    // AIS.
    /// Independent AIS Base Station
    IndependentAisBaseStation,
    /// Dependent AIS Base Station
    DependentAisBaseStation,
    /// Mobile Class A or B AIS Station
    AIS,
    /// AIS Aids to Navigation Station
    AisAton,
    /// AIS Receiving Station
    AisReceivingStation,
    /// AIS Limited Base Station
    AisLimitedBaseStation,
    /// AIS Transmitting Station
    AisTransmittingStation,
    /// AIS Simplex Repeater Station
    AisSimplexRepeater,
    /// AIS Physical Shore Station
    AisPhysicalShoreStation,

    // Satellite and radio positioning.
    /// Galileo
    Galileo,
    /// `BeiDou`
    BeiDou,
    /// `NavIC`
    NavIC,
    /// GLONASS
    Glonass,
    /// Combined Global Navigation Satellite Systems (GNSS)
    GNSS,
    /// Global Positioning System
    GPS,
    /// Quasi-Zenith Satellite System
    QuasiZenith,
    /// Loran C
    LoranC,
    /// General Electronic Positioning System
    PositioningSystem,

    // Navigation: heading, rudder, track control, speed, depth, radar, lights and the environment.
    /// General Heading Controller
    HeadingController,
    /// Magnetic Heading Controller
    MagneticHeadingController,
    /// Direction Finder
    DirectionFinder,
    /// Dynamic Position
    DynamicPosition,
    /// Magnetic Compass
    MagneticCompass,
    /// Fluxgate Compass
    FluxgateCompass,
    /// North-Seeking Gyro Compass
    NorthSeekingGyroCompass,
    /// Non-North-Seeking Gyro Compass
    NonNorthSeekingGyroCompass,
    /// Rudder Angle Indicator
    RudderAngle,
    /// Autonomous Navigation
    AutonomousNavigation,
    /// Navigation Light Controller
    NavigationLight,
    /// Night Vision
    NightVision,
    /// Radar
    Radar,
    /// Heave
    Heave,
    /// Inclinometer
    Inclinometer,
    /// Depth Sounder
    DepthSounder,
    /// Scanning Depth Sounder
    ScanningSounder,
    /// Track Control System
    TrackControlSystem,
    /// Turn Rate Indicator
    TurnRateIndicator,
    /// Doppler Speed Sensor
    DopplerVelocity,
    /// Magnetic Water Speed Sensor
    MagneticSpeed,
    /// Mechanical Water Speed Sensor
    MechanicalSpeed,
    /// Weather Instruments
    Weather,

    // Communications.
    /// Digital Selective Calling (DSC)
    DSC,
    /// General Data Receiver
    DataReceiver,
    /// Satellite
    Satellite,
    /// MF/HF Radio-Telephone
    MfHfTelephone,
    /// VHF Radio-Telephone
    VhfTelephone,
    /// Scanning Receiver
    ScanningReceiver,
    /// Duplex Repeater
    DuplexRepeater,
    /// VHF Data Exchange System (VDES) Terrestrial
    VdesTerrestrial,
    /// VHF Data Exchange System (VDES) Satellite
    VdesSatellite,
    /// VHF Data Exchange System (VDES) for Application Specific Messages
    VdesASM,

    // Safety: alarms, fire, doors, flooding and emergency.
    /// Alarm and Monitoring System
    AlarmMonitoring,
    /// Bridge Navigational Watch Alarm System
    BridgeWatchAlarm,
    /// Central Alert Management
    CentralAlarm,
    /// Emergency Position Indicating Beacon (EPIRB)
    EPIRB,
    /// Fire Door Controller or Monitor
    FireDoor,
    /// Fire Extinguisher System
    FireExtinguisher,
    /// Fire Detection Point
    FireDetection,
    /// Fire Sprinkler System
    FireSprinkler,
    /// Watertight Door Controller or Monitor
    WatertightDoor,
    /// Hull Door Controller or Monitor
    HullDoorMonitor,
    /// Hull Stress Monitoring
    HullStressMonitor,
    /// Water Level Detector
    WaterLevel,

    // Machinery and ship systems.
    /// Bilge Systems
    Bilge,
    /// Engine Room Monitoring Systems
    EngineRoom,
    /// Reefer Monitoring System
    ReeferMonitoring,
    /// Power Management System
    PowerManagement,
    /// Propulsion Control System
    PropulsionControl,
    /// Engine Control Console
    EngineConsole,
    /// Propulsion Boiler
    PropulsionBoiler,
    /// Auxiliary Boiler
    AuxiliaryBoiler,
    /// Electronic Governor System
    ElectronicGovernor,
    /// Propulsion Machinery or its Remote Control
    PropulsionMachinery,
    /// Steering Controller or Control System
    SteeringControl,
    /// Steering Gear or Engine
    Steering,

    // Time.
    /// Atomic Clock
    AtomicClock,
    /// Chronometer
    Chronometer,
    /// Quartz
    QuartzClock,
    /// Radio Update
    RadioClock,

    // Displays and integrated navigation.
    /// Electronic Chart System (ECS)
    ECS,
    /// Electronic Chart Display & Information System (ECDIS)
    ECDIS,
    /// Integrated Instrumentation
    IntegratedInstrumentation,
    /// Integrated Navigation
    IntegratedNavigation,

    // General systems.
    /// Integrated Autonomous
    IntegratedAutonomous,
    /// Multiplexer
    Multiplexer,
    /// Network Device
    NetworkDevice,
    /// Record Book
    RecordBook,
    /// Microprocessor Controller
    Microprocessor,
    /// Voyage Data Recorder
    VDR,
    /// Transducer
    Transducer,

    /// Any other talker, the user-configured `U0` to `U9` among them.
    Other([u8; 2]),
}

impl TryFrom<&str> for Talker {
    type Error = std::io::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "Invalid length"));
        }

        Ok(match value {
            "AB" => Talker::IndependentAisBaseStation,
            "AD" => Talker::DependentAisBaseStation,
            "AG" => Talker::HeadingController,
            "AI" => Talker::AIS,
            "AN" => Talker::AisAton,
            "AP" => Talker::MagneticHeadingController,
            "AR" => Talker::AisReceivingStation,
            "AS" => Talker::AisLimitedBaseStation,
            "AT" => Talker::AisTransmittingStation,
            "AX" => Talker::AisSimplexRepeater,
            "BI" => Talker::Bilge,
            "BN" => Talker::BridgeWatchAlarm,
            "CA" => Talker::CentralAlarm,
            "CD" => Talker::DSC,
            "CR" => Talker::DataReceiver,
            "CS" => Talker::Satellite,
            "CT" => Talker::MfHfTelephone,
            "CV" => Talker::VhfTelephone,
            "CX" => Talker::ScanningReceiver,
            "DF" => Talker::DirectionFinder,
            "DP" => Talker::DynamicPosition,
            "DU" => Talker::DuplexRepeater,
            "EC" => Talker::ECS,
            "EI" => Talker::ECDIS,
            "EP" => Talker::EPIRB,
            "ER" => Talker::EngineRoom,
            "FD" => Talker::FireDoor,
            "FE" => Talker::FireExtinguisher,
            "FR" => Talker::FireDetection,
            "FS" => Talker::FireSprinkler,
            "GA" => Talker::Galileo,
            "GB" => Talker::BeiDou,
            "GI" => Talker::NavIC,
            "GL" => Talker::Glonass,
            "GN" => Talker::GNSS,
            "GP" => Talker::GPS,
            "GQ" => Talker::QuasiZenith,
            "HC" => Talker::MagneticCompass,
            "HD" => Talker::HullDoorMonitor,
            "HE" => Talker::NorthSeekingGyroCompass,
            "HF" => Talker::FluxgateCompass,
            "HN" => Talker::NonNorthSeekingGyroCompass,
            "HS" => Talker::HullStressMonitor,
            "HV" => Talker::Heave,
            "IA" => Talker::IntegratedAutonomous,
            "II" => Talker::IntegratedInstrumentation,
            "IN" => Talker::IntegratedNavigation,
            "JA" => Talker::AlarmMonitoring,
            "JB" => Talker::ReeferMonitoring,
            "JC" => Talker::PowerManagement,
            "JD" => Talker::PropulsionControl,
            "JE" => Talker::EngineConsole,
            "JF" => Talker::PropulsionBoiler,
            "JG" => Talker::AuxiliaryBoiler,
            "JH" => Talker::ElectronicGovernor,
            "LC" => Talker::LoranC,
            "MX" => Talker::Multiplexer,
            "NA" => Talker::AutonomousNavigation,
            "ND" => Talker::NetworkDevice,
            "NL" => Talker::NavigationLight,
            "NV" => Talker::NightVision,
            "RA" => Talker::Radar,
            "RB" => Talker::RecordBook,
            "RC" => Talker::PropulsionMachinery,
            "RI" => Talker::RudderAngle,
            "RP" => Talker::Inclinometer,
            "SA" => Talker::AisPhysicalShoreStation,
            "SC" => Talker::SteeringControl,
            "SD" => Talker::DepthSounder,
            "SG" => Talker::Steering,
            "SN" => Talker::PositioningSystem,
            "SS" => Talker::ScanningSounder,
            "TC" => Talker::TrackControlSystem,
            "TI" => Talker::TurnRateIndicator,
            "UP" => Talker::Microprocessor,
            "VA" => Talker::VdesASM,
            "VD" => Talker::DopplerVelocity,
            "VM" => Talker::MagneticSpeed,
            "VR" => Talker::VDR,
            "VS" => Talker::VdesSatellite,
            "VT" => Talker::VdesTerrestrial,
            "VW" => Talker::MechanicalSpeed,
            "WD" => Talker::WatertightDoor,
            "WI" => Talker::Weather,
            "WL" => Talker::WaterLevel,
            "YX" => Talker::Transducer,
            "ZA" => Talker::AtomicClock,
            "ZC" => Talker::Chronometer,
            "ZQ" => Talker::QuartzClock,
            "ZV" => Talker::RadioClock,
            _ => {
                let value = value.as_bytes();
                Self::Other([value[0], value[1]])
            }
        })
    }
}

impl Display for Talker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Talker::IndependentAisBaseStation => "AB",
            Talker::DependentAisBaseStation => "AD",
            Talker::HeadingController => "AG",
            Talker::AIS => "AI",
            Talker::AisAton => "AN",
            Talker::MagneticHeadingController => "AP",
            Talker::AisReceivingStation => "AR",
            Talker::AisLimitedBaseStation => "AS",
            Talker::AisTransmittingStation => "AT",
            Talker::AisSimplexRepeater => "AX",
            Talker::Bilge => "BI",
            Talker::BridgeWatchAlarm => "BN",
            Talker::CentralAlarm => "CA",
            Talker::DSC => "CD",
            Talker::DataReceiver => "CR",
            Talker::Satellite => "CS",
            Talker::MfHfTelephone => "CT",
            Talker::VhfTelephone => "CV",
            Talker::ScanningReceiver => "CX",
            Talker::DirectionFinder => "DF",
            Talker::DynamicPosition => "DP",
            Talker::DuplexRepeater => "DU",
            Talker::ECS => "EC",
            Talker::ECDIS => "EI",
            Talker::EPIRB => "EP",
            Talker::EngineRoom => "ER",
            Talker::FireDoor => "FD",
            Talker::FireExtinguisher => "FE",
            Talker::FireDetection => "FR",
            Talker::FireSprinkler => "FS",
            Talker::Galileo => "GA",
            Talker::BeiDou => "GB",
            Talker::NavIC => "GI",
            Talker::Glonass => "GL",
            Talker::GNSS => "GN",
            Talker::GPS => "GP",
            Talker::QuasiZenith => "GQ",
            Talker::MagneticCompass => "HC",
            Talker::HullDoorMonitor => "HD",
            Talker::NorthSeekingGyroCompass => "HE",
            Talker::FluxgateCompass => "HF",
            Talker::NonNorthSeekingGyroCompass => "HN",
            Talker::HullStressMonitor => "HS",
            Talker::Heave => "HV",
            Talker::IntegratedAutonomous => "IA",
            Talker::IntegratedInstrumentation => "II",
            Talker::IntegratedNavigation => "IN",
            Talker::AlarmMonitoring => "JA",
            Talker::ReeferMonitoring => "JB",
            Talker::PowerManagement => "JC",
            Talker::PropulsionControl => "JD",
            Talker::EngineConsole => "JE",
            Talker::PropulsionBoiler => "JF",
            Talker::AuxiliaryBoiler => "JG",
            Talker::ElectronicGovernor => "JH",
            Talker::LoranC => "LC",
            Talker::Multiplexer => "MX",
            Talker::AutonomousNavigation => "NA",
            Talker::NetworkDevice => "ND",
            Talker::NavigationLight => "NL",
            Talker::NightVision => "NV",
            Talker::Radar => "RA",
            Talker::RecordBook => "RB",
            Talker::PropulsionMachinery => "RC",
            Talker::RudderAngle => "RI",
            Talker::Inclinometer => "RP",
            Talker::AisPhysicalShoreStation => "SA",
            Talker::SteeringControl => "SC",
            Talker::DepthSounder => "SD",
            Talker::Steering => "SG",
            Talker::PositioningSystem => "SN",
            Talker::ScanningSounder => "SS",
            Talker::TrackControlSystem => "TC",
            Talker::TurnRateIndicator => "TI",
            Talker::Microprocessor => "UP",
            Talker::VdesASM => "VA",
            Talker::DopplerVelocity => "VD",
            Talker::MagneticSpeed => "VM",
            Talker::VDR => "VR",
            Talker::VdesSatellite => "VS",
            Talker::VdesTerrestrial => "VT",
            Talker::MechanicalSpeed => "VW",
            Talker::WatertightDoor => "WD",
            Talker::Weather => "WI",
            Talker::WaterLevel => "WL",
            Talker::Transducer => "YX",
            Talker::AtomicClock => "ZA",
            Talker::Chronometer => "ZC",
            Talker::QuartzClock => "ZQ",
            Talker::RadioClock => "ZV",
            Talker::Other(val) => str::from_utf8(val).map_err(|_| std::fmt::Error)?,
        };
        f.write_str(s)
    }
}

#[derive(Clone, Debug)]
pub struct Latitude(pub Angle);

impl Latitude {
    /// The latitude from its degrees and minutes and its hemisphere, `None` when either is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(latitude: &str, latitude_ord: &str) -> Result<Option<Latitude>, Error> {
        if latitude.len() < 4 || latitude_ord.is_empty() {
            return Ok(None);
        }
        let (degrees, minutes) = latitude.split_at(2);
        let degrees = f64::from_str(degrees).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid degrees"))?;
        let minutes = f64::from_str(minutes).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid minutes"))?;
        let latitude = degrees + minutes / 60.0;
        let latitude = match latitude_ord {
            "N" => latitude,
            "S" => -latitude,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid ordinal")),
        };
        let latitude = Angle::new::<degree>(latitude);
        Ok(Some(Latitude(latitude)))
    }
}
impl Display for Latitude {
    /// The latitude as `ddmm.mmmm,N` or `S`.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_degrees_minutes(f, self.0, 2, 'N', 'S')
    }
}

/// Writes an angle as whole degrees with `width` digits and minutes to `1/10_000`, then the
/// hemisphere: `positive` for north or east and `negative` for south or west.
fn write_degrees_minutes(
    f: &mut Formatter<'_>,
    angle: Angle,
    width: usize,
    positive: char,
    negative: char,
) -> std::fmt::Result {
    let degrees = angle.get::<degree>();
    // Round to the precision written before splitting, so the minutes never round up to 60.
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a whole, positive angle")]
    let units = (degrees.abs() * 60.0 * 10_000.0).round() as u64;
    let (whole, minutes) = (units / 600_000, units % 600_000);
    let hemisphere = if degrees >= 0.0 || units == 0 { positive } else { negative };
    write!(f, "{whole:0width$}{:02}.{:04},{hemisphere}", minutes / 10_000, minutes % 10_000)
}

#[derive(Clone, Debug)]
pub struct Longitude(pub Angle);
impl Longitude {
    /// The longitude from its degrees and minutes and its hemisphere, `None` when either is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(longitude: &str, longitude_ord: &str) -> Result<Option<Longitude>, Error> {
        if longitude.len() < 5 || longitude_ord.is_empty() {
            return Ok(None);
        }
        let (degrees, minutes) = longitude.split_at(3);
        let degrees = f64::from_str(degrees).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid degrees"))?;
        let minutes = f64::from_str(minutes).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid minutes"))?;
        let longitude = degrees + minutes / 60.0;
        let longitude = match longitude_ord {
            "E" => longitude,
            "W" => -longitude,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid ordinal")),
        };
        let longitude = Angle::new::<degree>(longitude);
        Ok(Some(Longitude(longitude)))
    }
}

impl Display for Longitude {
    /// The longitude as `dddmm.mmmm,E` or `W`.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write_degrees_minutes(f, self.0, 3, 'E', 'W')
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub latitude: Latitude,
    pub longitude: Longitude,
}
impl Position {
    /// The position from its latitude and longitude fields, `None` when either is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(
        latitude: &str,
        latitude_ord: &str,
        longitude: &str,
        longitude_ord: &str,
    ) -> Result<Option<Position>, Error> {
        let Some(latitude) = Latitude::parse_str(latitude, latitude_ord)? else { return Ok(None) };
        let Some(longitude) = Longitude::parse_str(longitude, longitude_ord)? else { return Ok(None) };
        Ok(Some(Position { latitude, longitude }))
    }
}

#[derive(Clone, Debug)]
pub struct SpeedOverGround(pub Velocity);
impl SpeedOverGround {
    /// The speed in knots, `None` when the field is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(speed: &str) -> Result<Option<SpeedOverGround>, Error> {
        if speed.is_empty() {
            return Ok(None);
        }
        let sog = f64::from_str(speed).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid sog"))?;
        Ok(Some(SpeedOverGround(Velocity::new::<knot>(sog))))
    }
}

#[derive(Clone, Debug)]
pub struct CourseOverGround(pub Angle);
impl CourseOverGround {
    /// The course in degrees true, `None` when the field is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(cog: &str) -> Result<Option<CourseOverGround>, Error> {
        if cog.is_empty() {
            return Ok(None);
        }
        let cog = f64::from_str(cog).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid cog"))?;
        Ok(Some(CourseOverGround(Angle::new::<degree>(cog))))
    }
}
impl Display for CourseOverGround {
    /// The course in degrees true to a tenth of a degree, from 0.0 to 359.9.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "0 to 3599 tenths")]
        let tenths = (self.0.get::<degree>() * 10.0).round().rem_euclid(3600.0) as u32;
        write!(f, "{}.{}", tenths / 10, tenths % 10)
    }
}
#[derive(Clone, Debug)]
pub struct MagneticVariation(pub Angle);
impl MagneticVariation {
    /// The variation in degrees east or west, `None` when either field is empty.
    ///
    /// # Errors
    ///
    /// An [`ErrorKind::InvalidData`] error for a field that isn't a number.
    pub fn parse_str(variation: &str, variation_ord: &str) -> Result<Option<MagneticVariation>, Error> {
        if variation.is_empty() || variation_ord.is_empty() {
            return Ok(None);
        }
        let variation = f64::from_str(variation).map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid degrees"))?;
        let variation = match variation_ord {
            "E" => variation,
            "W" => -variation,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid ordinal")),
        };
        let variation = Angle::new::<degree>(variation);
        Ok(Some(MagneticVariation(variation)))
    }
}
impl Display for MagneticVariation {
    /// The variation to a tenth of a degree, then `E` or `W`.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let degrees = self.0.get::<degree>();
        // Round before choosing the direction, so a variation that rounds to zero is not west.
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "a whole, positive angle")]
        let tenths = (degrees.abs() * 10.0).round() as u64;
        let direction = if degrees >= 0.0 || tenths == 0 { 'E' } else { 'W' };
        write!(f, "{}.{},{direction}", tenths / 10, tenths % 10)
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum PositioningSystemStatus {
    Valid = b'A',
    Warning = b'V',
}
impl Display for PositioningSystemStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(*self as u8 as char)
    }
}
impl TryFrom<&str> for PositioningSystemStatus {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "A" => PositioningSystemStatus::Valid,
            "V" => PositioningSystemStatus::Warning,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown PositioningSystemStatus")),
        })
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum PositioningSystemMode {
    NoFix = b'N',
    Autonomous = b'A',
    Differential = b'D',
    Estimated = b'E',
    Manual = b'M',
    Precise = b'P',
    FixedRTK = b'R',
    FloatRTK = b'F',
    Simulator = b'S',
}
impl Display for PositioningSystemMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(*self as u8 as char)
    }
}
impl TryFrom<&str> for PositioningSystemMode {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "N" => PositioningSystemMode::NoFix,
            "A" => PositioningSystemMode::Autonomous,
            "D" => PositioningSystemMode::Differential,
            "E" => PositioningSystemMode::Estimated,
            "M" => PositioningSystemMode::Manual,
            "P" => PositioningSystemMode::Precise,
            "R" => PositioningSystemMode::FixedRTK,
            "F" => PositioningSystemMode::FloatRTK,
            "S" => PositioningSystemMode::Simulator,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown PositioningSystemMode")),
        })
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum NavigationalStatus {
    Safe = b'S',
    Caution = b'C',
    Unsafe = b'U',
    NotValid = b'V',
}
impl Display for NavigationalStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char(*self as u8 as char)
    }
}
impl TryFrom<&str> for NavigationalStatus {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "S" => NavigationalStatus::Safe,
            "C" => NavigationalStatus::Caution,
            "U" => NavigationalStatus::Unsafe,
            "V" => NavigationalStatus::NotValid,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown NavigationalStatus")),
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AisChannel {
    A,
    B,
    Other(char),
}
impl TryFrom<&str> for AisChannel {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut chars = value.chars();
        Ok(match (chars.next(), chars.next()) {
            (Some('A'), None) => AisChannel::A,
            (Some('B'), None) => AisChannel::B,
            (Some(ch), None) => AisChannel::Other(ch),
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid AisChannel")),
        })
    }
}
impl Display for AisChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ch = match self {
            AisChannel::A => 'A',
            AisChannel::B => 'B',
            AisChannel::Other(ch) => *ch,
        };
        f.write_char(ch)
    }
}

/// The AIS channel an ABM or BBM message is to be transmitted on, sent as a number from 0 to 3.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TransmitChannel {
    /// No preference (0).
    NoPreference,
    /// Channel A (1).
    A,
    /// Channel B (2).
    B,
    /// Both channels A and B (3).
    Both,
}
impl TryFrom<&str> for TransmitChannel {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "0" => TransmitChannel::NoPreference,
            "1" => TransmitChannel::A,
            "2" => TransmitChannel::B,
            "3" => TransmitChannel::Both,
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid TransmitChannel")),
        })
    }
}
impl Display for TransmitChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ch = match self {
            TransmitChannel::NoPreference => '0',
            TransmitChannel::A => '1',
            TransmitChannel::B => '2',
            TransmitChannel::Both => '3',
        };
        f.write_char(ch)
    }
}

/// A Maritime Mobile Station Identifier.
#[derive(Debug, Copy, Clone)]
pub struct MMSI(pub u32);
impl Display for MMSI {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(feature = "ais")]
impl From<ais_message::types::MMSI> for MMSI {
    fn from(mmsi: ais_message::types::MMSI) -> Self {
        Self(mmsi.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every talker ID reads and writes the same, the named ones and the others alike: no two
    /// talkers write one ID.
    #[test]
    fn every_talker_writes_the_id_it_reads() {
        let symbols = (b'A'..=b'Z').chain(b'0'..=b'9');
        for first in symbols.clone() {
            for second in symbols.clone() {
                let id = String::from_utf8(vec![first, second]).unwrap();
                let talker = Talker::try_from(id.as_str()).unwrap();
                assert_eq!(talker.to_string(), id, "{talker:?}");
            }
        }
        assert!(matches!(Talker::try_from("AS").unwrap(), Talker::AisLimitedBaseStation));
        assert!(matches!(Talker::try_from("AT").unwrap(), Talker::AisTransmittingStation));
    }
}
