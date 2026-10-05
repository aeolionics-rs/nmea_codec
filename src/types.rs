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
    /// AB: Independent AIS Base Station
    IndependentAisBaseStation,
    /// AD: Dependent AIS Base Station
    DependentAisBaseStation,
    /// AG: HEADING TRACK CONTROLLER (Autopilot), General
    HeadingController,
    /// AP: HEADING TRACK CONTROLLER (Autopilot), Magnetic
    HeadingControllerMagnetic,
    /// AI: Mobile Class A or B AIS Station
    AIS,
    /// AN: AIS Aids to Navigation Station
    AisAton,
    /// AR: AIS Receiving Station
    AisReceivingStation,
    /// AS: AIS Station (ITU_R M.1371, "Limited Base Station")
    AisLimitedBaseStation,
    /// AT: AIS Transmitting Station
    AisTransmittingStation,
    /// AX: AIS Simplex Repeater Station
    AisSimplexRepeater,
    /// BI: Bilge Systems
    Bilge,
    /// BN: Bridge Navigational Watch Alarm System
    BridgeAlarm,
    /// CA: Central Alarm Management
    CentralAlarm,
    /// CD: Digital Selective Calling (DSC)
    DSC,
    /// CR: Data Receiver
    DataReceiver,
    /// CS: Satellite
    Satellite,
    /// CT: Radio-Telephone (MF/HF)
    MfHfTelephone,
    /// CV: Radio-Telephone (VHF)
    VhfTelephone,
    /// CX: Scanning Receiver
    ScanningReceiver,
    /// DF: Direction Finder
    DirectionFinder,
    /// DU: Duplex Repeater Station
    DuplexRepeater,
    /// DP: Dynamic Position
    DynamicPosition,
    /// EC: Electronic Chart System (ECS)
    ECS,
    /// EI: Electronic Chart Display & Information System (ECDIS)
    ECDIS,
    /// EP: Emergency Position Indicating Beacon (EPIRB)
    EPIRB,
    /// ER: Engine Room Monitoring Systems
    EngineRoom,
    /// FD: Fire Door Controller/Monitoring Point
    FireDoor,
    /// FE: Fire Extinguisher System
    FireExtinguisher,
    /// FR: Fire Detection Point
    FireDetection,
    /// FS: Fire Sprinkler System
    FireSprinkler,
    /// GA: Galileo Positioning System
    Galileo,
    /// GB: BeiDou Satellite System (BDS) Receiver
    BeiDou,
    /// GI: NavIC Indian Regional Navigation Satellite System (IRNSS) Receiver
    NavIC,
    /// GL: GLONASS Receiver
    Glonass,
    /// GN: Global Navigation Satellite System (GNSS) Receiver
    GNSS,
    /// GP: Global Positioning System (GPS) Receiver
    GPS,
    /// GQ: Quasi-Zenith Satellite System (QZSS) Receiver
    QuasiZenith,
    /// HC: Compass, Magnetic
    MagneticCompass,
    /// HE: Gyro, North Seeking
    NorthSeekingGyroCompass,
    /// HF: Fluxgate
    FluxgateCompass,
    /// HN: Gyro, Non-North Seeking
    NonNorthSeekingGyroCompass,
    /// HD: Hull Door Controller/Monitoring Panel
    HullDoorMonitor,
    /// HS: Hull Stress Monitoring
    HullStressMonitor,
    /// HV: Motion Reference Unit - Heave
    Heave,
    /// IA: Integrated Autonomous
    IntegratedAutonomous,
    /// II: Integrated Instrumentation
    IntegratedInstrumentation,
    /// IN: Integrated Navigation
    IntegratedNavigation,
    /// JA: Alarm and Monitoring System
    AlarmMonitoring,
    /// JB: Reefer Monitoring System
    ReeferMonitoring,
    /// JC: Power Management System
    PowerManagement,
    /// JD: Propulsion Control System
    PropulsionControl,
    /// JE: Engine Control Console
    EngineConsole,
    /// JF: Propulsion Boiler
    PropulsionBoiler,
    /// JG: Auxiliary Boiler
    AuxiliaryBoiler,
    /// JH: Electronic Governor System
    ElectronicGovernor,
    /// LC: Loran C
    LoranC,
    /// MX: Multiplexer
    Multiplexer,
    /// NA: Navigation Autonomous
    AutonomousNavigation,
    /// ND: Network Device
    NetworkDevice,
    /// NV: Night Vision
    NightVision,
    /// NL: Navigation Light Controller
    NavigationLight,
    /// RA: Radar and/or Radar Plotting
    Radar,
    /// RB: Record Book
    RecordBook,
    /// RC: Propulsion Machinery, including Remote Control
    PropulsionMachinery,
    /// RI: Rudder Angle Indicator
    RudderAngle,
    /// RP: Inclinometer
    Inclinometer,
    /// SA: Physical Shore AIS Station
    AisPhysicalShoreStation,
    /// SC: Steering Control System/Device
    SteeringControl,
    /// SD: Sounder, depth
    DepthSounder,
    /// SG: Steering Gear/Steering Engine
    Steering,
    /// SN: Electronic Positioning System, other/general
    PositioningSystem,
    /// SS: Sounder, scanning
    ScanningSounder,
    /// TC: Track Control System
    TrackControlSystem,
    /// TI: Turn Rate Indicator
    TurnRateIndicator,
    /// UP: Microprocessor Controller
    Microprocessor,
    /// VD: Doppler, other/general
    DopplerVelocity,
    /// VM: Speed Log, Water, Magnetic
    MagneticSpeed,
    /// VW: Speed Log, Water Mechanical
    MechanicalSpeed,
    /// VA: VHF Data Exchange System (VDES) ASM
    VdesASM,
    /// VS: VHF Data Exchange System (VDES) Satellite
    VdesSatellite,
    /// VT: VHF Data Exchange System (VDES) Terrestrial
    VdesTerrestrial,
    /// VD: Voyage Data Recorder
    VDR,
    /// WD: Watertight Door Controller/Monitoring Panel
    WatertightDoor,
    /// WI: Weather Instruments
    Weather,
    /// WL: Water Level Detection Systems
    WaterLevel,
    /// YX: Transducer
    Transducer,
    /// ZA: Atomic Clock
    AtomicClock,
    /// ZC: Chronometer
    Chronometer,
    /// ZQ: Quartz
    QuartzClock,
    /// ZV: Radio Update
    RadioClock,

    User0,
    User1,
    User2,
    User3,
    User4,
    User5,
    User6,
    User7,
    User8,
    User9,
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
            "AP" => Talker::HeadingControllerMagnetic,
            "AI" => Talker::AIS,
            "AN" => Talker::AisAton,
            "AR" => Talker::AisReceivingStation,
            "AS" => Talker::AisLimitedBaseStation,
            "AT" => Talker::AisTransmittingStation,
            "AX" => Talker::AisSimplexRepeater,
            "BI" => Talker::Bilge,
            "BN" => Talker::BridgeAlarm,
            "CA" => Talker::CentralAlarm,
            "CD" => Talker::DSC,
            "CR" => Talker::DataReceiver,
            "CS" => Talker::Satellite,
            "CT" => Talker::MfHfTelephone,
            "CV" => Talker::VhfTelephone,
            "CX" => Talker::ScanningReceiver,
            "DF" => Talker::DirectionFinder,
            "DU" => Talker::DuplexRepeater,
            "DP" => Talker::DynamicPosition,
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
            "HE" => Talker::NorthSeekingGyroCompass,
            "HF" => Talker::FluxgateCompass,
            "HN" => Talker::NonNorthSeekingGyroCompass,
            "HD" => Talker::HullDoorMonitor,
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
            "NV" => Talker::NightVision,
            "NL" => Talker::NavigationLight,
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
            "VD" => Talker::DopplerVelocity,
            "VM" => Talker::MagneticSpeed,
            "VW" => Talker::MechanicalSpeed,
            "VA" => Talker::VdesASM,
            "VS" => Talker::VdesSatellite,
            "VT" => Talker::VdesTerrestrial,
            "VR" => Talker::VDR,
            "WD" => Talker::WatertightDoor,
            "WI" => Talker::Weather,
            "WL" => Talker::WaterLevel,
            "YX" => Talker::Transducer,
            "ZA" => Talker::AtomicClock,
            "ZC" => Talker::Chronometer,
            "ZQ" => Talker::QuartzClock,
            "ZV" => Talker::RadioClock,

            "U0" => Talker::User0,
            "U1" => Talker::User1,
            "U2" => Talker::User2,
            "U3" => Talker::User3,
            "U4" => Talker::User4,
            "U5" => Talker::User5,
            "U6" => Talker::User6,
            "U7" => Talker::User7,
            "U8" => Talker::User8,
            "U9" => Talker::User9,

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
            Talker::HeadingControllerMagnetic => "AP",
            Talker::AIS => "AI",
            Talker::AisAton => "AN",
            Talker::AisReceivingStation => "AR",
            Talker::AisLimitedBaseStation => "AT",
            Talker::AisTransmittingStation => "AT",
            Talker::AisSimplexRepeater => "AX",
            Talker::Bilge => "BI",
            Talker::BridgeAlarm => "BN",
            Talker::CentralAlarm => "CA",
            Talker::DSC => "CD",
            Talker::DataReceiver => "CR",
            Talker::Satellite => "CS",
            Talker::MfHfTelephone => "CT",
            Talker::VhfTelephone => "CV",
            Talker::ScanningReceiver => "CX",
            Talker::DirectionFinder => "DF",
            Talker::DuplexRepeater => "DU",
            Talker::DynamicPosition => "DP",
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
            Talker::NorthSeekingGyroCompass => "HE",
            Talker::FluxgateCompass => "HF",
            Talker::NonNorthSeekingGyroCompass => "HN",
            Talker::HullDoorMonitor => "HD",
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
            Talker::NightVision => "NV",
            Talker::NavigationLight => "NL",
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
            Talker::DopplerVelocity => "VD",
            Talker::MagneticSpeed => "VM",
            Talker::MechanicalSpeed => "VW",
            Talker::VdesASM => "VA",
            Talker::VdesSatellite => "VS",
            Talker::VdesTerrestrial => "VT",
            Talker::VDR => "VR",
            Talker::WatertightDoor => "WD",
            Talker::WaterLevel => "WL",
            Talker::Transducer => "YX",
            Talker::AtomicClock => "ZA",
            Talker::Chronometer => "ZC",
            Talker::QuartzClock => "ZQ",
            Talker::RadioClock => "ZV",
            Talker::Weather => "WI",

            Talker::User0 => "U0",
            Talker::User1 => "U1",
            Talker::User2 => "U2",
            Talker::User3 => "U3",
            Talker::User4 => "U4",
            Talker::User5 => "U5",
            Talker::User6 => "U6",
            Talker::User7 => "U7",
            Talker::User8 => "U8",
            Talker::User9 => "U9",
            Talker::Other(val) => str::from_utf8(val).map_err(|_| std::fmt::Error)?,
        };
        f.write_str(s)
    }
}

#[derive(Clone, Debug)]
pub struct Latitude(pub Angle);

impl Latitude {
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
        write_degrees_minutes(f, &self.0, 2, 'N', 'S')
    }
}

/// Writes an angle as whole degrees with `width` digits and minutes to 1/10_000, then the
/// hemisphere: `positive` for north or east and `negative` for south or west.
fn write_degrees_minutes(f: &mut Formatter<'_>, angle: &Angle, width: usize, positive: char, negative: char) -> std::fmt::Result {
    let degrees = angle.get::<degree>();
    // Round to the precision written before splitting, so the minutes never round up to 60.
    let units = (degrees.abs() * 60.0 * 10_000.0).round() as u64;
    let (whole, minutes) = (units / 600_000, units % 600_000);
    let hemisphere = if degrees >= 0.0 || units == 0 { positive } else { negative };
    write!(f, "{whole:0width$}{:02}.{:04},{hemisphere}", minutes / 10_000, minutes % 10_000)
}

#[derive(Clone, Debug)]
pub struct Longitude(pub Angle);
impl Longitude {
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
        write_degrees_minutes(f, &self.0, 3, 'E', 'W')
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub latitude: Latitude,
    pub longitude: Longitude,
}
impl Position {
    pub fn parse_str(latitude: &str, latitude_ord: &str, longitude: &str, longitude_ord: &str) -> Result<Option<Position>, Error> {
        let Some(latitude) = Latitude::parse_str(latitude, latitude_ord)? else { return Ok(None) };
        let Some(longitude) = Longitude::parse_str(longitude, longitude_ord)? else { return Ok(None) };
        Ok(Some(Position { latitude, longitude }))
    }
}

#[derive(Clone, Debug)]
pub struct SpeedOverGround(pub Velocity);
impl SpeedOverGround {
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
        let tenths = (self.0.get::<degree>() * 10.0).round().rem_euclid(3600.0) as u32;
        write!(f, "{}.{}", tenths / 10, tenths % 10)
    }
}
#[derive(Clone, Debug)]
pub struct MagneticVariation(pub Angle);
impl MagneticVariation {
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
