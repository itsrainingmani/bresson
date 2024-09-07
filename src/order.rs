use std::collections::BTreeSet;

use exif::Tag;

pub const EXIF_FIELDS_ORDERED: [Tag; 67] = [
    Tag::Make,
    Tag::Model,
    Tag::DateTimeOriginal,
    Tag::ExposureTime,
    Tag::ExposureBiasValue,
    Tag::FNumber,
    Tag::PhotographicSensitivity,
    Tag::FocalLength,
    Tag::WhiteBalance,
    Tag::MeteringMode,
    Tag::GPSLatitude,
    Tag::GPSLatitudeRef,
    Tag::GPSLongitude,
    Tag::GPSLongitudeRef,
    Tag::LensModel,
    Tag::Flash,
    Tag::Orientation,
    Tag::XResolution,
    Tag::YResolution,
    Tag::ResolutionUnit,
    Tag::Software,
    Tag::DateTime,
    Tag::YCbCrPositioning,
    Tag::ExposureProgram,
    Tag::ExifVersion,
    Tag::DateTimeDigitized,
    Tag::OffsetTime,
    Tag::OffsetTimeOriginal,
    Tag::OffsetTimeDigitized,
    Tag::ComponentsConfiguration,
    Tag::ShutterSpeedValue,
    Tag::ApertureValue,
    Tag::BrightnessValue,
    Tag::SubjectArea,
    Tag::MakerNote,
    Tag::SubSecTimeOriginal,
    Tag::SubSecTimeDigitized,
    Tag::FlashpixVersion,
    Tag::ColorSpace,
    Tag::PixelXDimension,
    Tag::PixelYDimension,
    Tag::SensingMethod,
    Tag::SceneType,
    Tag::ExposureMode,
    Tag::DigitalZoomRatio,
    Tag::FocalLengthIn35mmFilm,
    Tag::SceneCaptureType,
    Tag::LensSpecification,
    Tag::LensMake,
    Tag::CompositeImage,
    Tag::GPSAltitudeRef,
    Tag::GPSAltitude,
    Tag::GPSTimeStamp,
    Tag::GPSSpeedRef,
    Tag::GPSSpeed,
    Tag::GPSImgDirectionRef,
    Tag::GPSImgDirection,
    Tag::GPSDestBearingRef,
    Tag::GPSDestBearing,
    Tag::GPSDateStamp,
    Tag::GPSHPositioningError,
    Tag::Compression,
    Tag::XResolution,
    Tag::YResolution,
    Tag::ResolutionUnit,
    Tag::JPEGInterchangeFormat,
    Tag::JPEGInterchangeFormatLength,
];

#[derive(Debug, Clone)]
pub struct OrderedTags {
    pub tags: BTreeSet<Tag>,
}

impl OrderedTags {
    pub fn new() -> Self {
        Self {
            tags: BTreeSet::from(EXIF_FIELDS_ORDERED),
        }
    }
}
