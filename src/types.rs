use std::fmt::{Display, Formatter, Write};
use uom::si::angle::degree;
use uom::si::f64::{Angle, Velocity};

#[derive(Debug, Clone, Copy)]
pub enum Talker {
    HeadingController,
    HeadingControllerMagnetic,
    AIS,
    Bilge,
    BridgeAlarm,
    DSC,
    DataReceiver,
    Satellite,
    MfHfTelephone,
    VhfTelephone,
    ScanningReceiver,
    DirectionFinder,
    DuplexRepeater,
    ECS,
    ECDIS,
    EPIRB,
    EngineRoom,
    FireDoor,
    FireExtinguisher,
    FireDetection,
    FireSprinkler,
    GPS,
    Galileo,
    Glonass,
    GNSS,
    MagneticCompass,
    NorthSeekingGyroCompass,
    FluxgateCompass,
    NonNorthSeekingGyroCompass,
    HullDoorMonitor,
    HullStressMonitor,
    IntegratedInstrumentation,
    IntegratedNavigation,
    LoranC,
    NavigationLight,
    Proprietary([u8; 3]),
    Radar,
    Propulsion,
    DepthSounder,
    Steering,
    PositioningSystem,
    ScanningSounder,
    TurnRateIndicator,
    Microprocessor,
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
    DopplerVelocity,
    MagneticSpeed,
    MechanicalSpeed,
    VDR,
    WatertightDoor,
    WaterLevel,
    Transducer,
    AtomicClock,
    Chronometer,
    QuartzClock,
    RadioClock,
    Weather,
    Other([u8; 2]),
}

impl Display for Talker {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Talker::HeadingController => "AG",
            Talker::HeadingControllerMagnetic => "AP",
            Talker::AIS => "AI",
            Talker::Bilge => "BI",
            Talker::BridgeAlarm => "BN",
            Talker::DSC => "CD",
            Talker::DataReceiver => "CR",
            Talker::Satellite => "CS",
            Talker::MfHfTelephone => "CT",
            Talker::VhfTelephone => "CV",
            Talker::ScanningReceiver => "CX",
            Talker::DirectionFinder => "DF",
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
            Talker::Glonass => "GL",
            Talker::GPS => "GP",
            Talker::GNSS => "GN",
            Talker::MagneticCompass => "HC",
            Talker::NorthSeekingGyroCompass => "HE",
            Talker::FluxgateCompass => "HF",
            Talker::NonNorthSeekingGyroCompass => "HN",
            Talker::HullDoorMonitor => "HD",
            Talker::HullStressMonitor => "HS",
            Talker::IntegratedInstrumentation => "II",
            Talker::IntegratedNavigation => "IN",
            Talker::LoranC => "LC",
            Talker::NavigationLight => "NL",
            Talker::Radar => "RA",
            Talker::Propulsion => "RC",
            Talker::DepthSounder => "SD",
            Talker::Steering => "SG",
            Talker::PositioningSystem => "SN",
            Talker::ScanningSounder => "SS",
            Talker::TurnRateIndicator => "TI",
            Talker::Microprocessor => "UP",
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
            Talker::DopplerVelocity => "VD",
            Talker::MagneticSpeed => "VM",
            Talker::MechanicalSpeed => "VW",
            Talker::VDR => "VR",
            Talker::WatertightDoor => "WD",
            Talker::WaterLevel => "WL",
            Talker::Transducer => "YX",
            Talker::AtomicClock => "ZA",
            Talker::Chronometer => "ZC",
            Talker::QuartzClock => "ZQ",
            Talker::RadioClock => "ZV",
            Talker::Weather => "WI",
            Talker::Proprietary(val) => {
                return write!(f, "P{}", str::from_utf8(val).map_err(|_| std::fmt::Error)?);
            },
            Talker::Other(val) => str::from_utf8(val).map_err(|_| std::fmt::Error)?,
        };
        f.write_str(s)
    }
}

pub struct Latitude(pub Angle);

impl Display for Latitude {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let degrees = self.0.get::<degree>().abs();
        write!(f, "{:02}{:02},{}", degrees.trunc(), degrees.fract() * 60.0, if self.0.is_sign_positive() { 'N' } else { 'S' })
    }
}

pub struct Longitude(pub Angle);

impl Display for Longitude {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let degrees = self.0.get::<degree>().abs();
        write!(f, "{:02}{:02},{}", degrees.trunc(), degrees.fract() * 60.0, if self.0.is_sign_positive() { 'E' } else { 'W' })
    }
}

pub struct Position {
    pub latitude: Latitude,
    pub longitude: Longitude,
}

pub struct SpeedOverGround(pub Velocity);

pub struct CourseOverGround(pub Angle);

pub struct MagneticVariation(pub Angle);

impl Display for MagneticVariation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.0.get::<degree>().abs(), if self.0.is_sign_positive() { 'E' } else { 'W' })
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

#[derive(Debug, Copy, Clone)]
pub enum AisChannel {
    A,
    B,
    Other(char)
}
impl Display for AisChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let ch = match self {
            AisChannel::A => 'A',
            AisChannel::B => 'B',
            AisChannel::Other(ch) => *ch
        };
        f.write_char(ch)
    }
}
