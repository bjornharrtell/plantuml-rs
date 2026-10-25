//! Conversion of legacy `skinparam` settings to styles, ported from `FromSkinparamToStyle`.

use super::names::{PName, SName};
use super::value::{Specificity, Value};
use super::{Style, StyleBuilder, StyleQuery};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

struct Data {
    style_names: Vec<SName>,
    property: PName,
}

type Knowledge = HashMap<String, Vec<Data>>;

fn add_convert(k: &mut Knowledge, skinparam: &str, property: PName, names: &[SName]) {
    k.entry(skinparam.to_lowercase()).or_default().push(Data {
        style_names: names.to_vec(),
        property,
    });
}

fn add_con_font(k: &mut Knowledge, skinparam: &str, names: &[SName]) {
    add_convert(k, &format!("{skinparam}FontSize"), PName::FontSize, names);
    add_convert(k, &format!("{skinparam}FontStyle"), PName::FontStyle, names);
    add_convert(k, &format!("{skinparam}FontColor"), PName::FontColor, names);
    add_convert(k, &format!("{skinparam}FontName"), PName::FontName, names);
}

fn add_magic(k: &mut Knowledge, sname: SName) {
    let clean = sname.key();
    add_convert(
        k,
        &format!("{clean}BackgroundColor"),
        PName::BackGroundColor,
        &[sname],
    );
    add_convert(
        k,
        &format!("{clean}BorderColor"),
        PName::LineColor,
        &[sname],
    );
    add_convert(
        k,
        &format!("{clean}BorderThickness"),
        PName::LineThickness,
        &[sname],
    );
    add_convert(
        k,
        &format!("{clean}RoundCorner"),
        PName::RoundCorner,
        &[sname],
    );
    add_convert(
        k,
        &format!("{clean}DiagonalCorner"),
        PName::DiagonalCorner,
        &[sname],
    );
    add_convert(
        k,
        &format!("{clean}BorderStyle"),
        PName::LineStyle,
        &[sname],
    );
    add_con_font(k, clean, &[sname]);
    add_convert(k, &format!("{clean}Shadowing"), PName::Shadowing, &[sname]);
    add_convert(
        k,
        &format!("{clean}StereotypeFontSize"),
        PName::FontSize,
        &[SName::Stereotype, sname],
    );
    add_convert(
        k,
        &format!("{clean}StereotypeFontStyle"),
        PName::FontStyle,
        &[SName::Stereotype, sname],
    );
    add_convert(
        k,
        &format!("{clean}StereotypeFontColor"),
        PName::FontColor,
        &[SName::Stereotype, sname],
    );
    add_convert(
        k,
        &format!("{clean}StereotypeFontName"),
        PName::FontName,
        &[SName::Stereotype, sname],
    );
}

static KNOWLEDGE: LazyLock<Knowledge> = LazyLock::new(|| {
    let mut k = Knowledge::new();
    let kn = &mut k;
    add_convert(
        kn,
        "participantClickableBackgroundColor",
        PName::BackGroundColor,
        &[SName::Participant, SName::Clickable],
    );
    add_convert(
        kn,
        "participantClickableBorderColor",
        PName::LineColor,
        &[SName::Participant, SName::Clickable],
    );
    add_magic(kn, SName::Participant);
    add_magic(kn, SName::Boundary);
    add_magic(kn, SName::Control);
    add_magic(kn, SName::Collections);
    add_magic(kn, SName::Actor);
    add_magic(kn, SName::Database);
    add_magic(kn, SName::Entity);
    add_con_font(kn, "header", &[SName::Document, SName::Header]);
    add_con_font(kn, "footer", &[SName::Document, SName::Footer]);
    add_con_font(kn, "caption", &[SName::Document, SName::Caption]);
    add_convert(kn, "defaultFontSize", PName::FontSize, &[SName::Element]);
    add_convert(
        kn,
        "sequenceStereotypeFontSize",
        PName::FontSize,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontStyle",
        PName::FontStyle,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontColor",
        PName::FontColor,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontName",
        PName::FontName,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "SequenceReferenceBorderColor",
        PName::LineColor,
        &[SName::Reference],
    );
    add_convert(
        kn,
        "SequenceReferenceBorderColor",
        PName::LineColor,
        &[SName::ReferenceHeader],
    );
    add_convert(
        kn,
        "SequenceReferenceBackgroundColor",
        PName::BackGroundColor,
        &[SName::Reference],
    );
    add_convert(
        kn,
        "sequenceReferenceHeaderBackgroundColor",
        PName::BackGroundColor,
        &[SName::ReferenceHeader],
    );
    add_con_font(kn, "sequenceReference", &[SName::Reference]);
    add_con_font(kn, "sequenceReference", &[SName::ReferenceHeader]);
    add_convert(
        kn,
        "sequenceGroupBorderThickness",
        PName::LineThickness,
        &[SName::Group],
    );
    add_convert(
        kn,
        "SequenceGroupBorderColor",
        PName::LineColor,
        &[SName::Group],
    );
    add_convert(
        kn,
        "SequenceGroupBorderColor",
        PName::LineColor,
        &[SName::GroupHeader],
    );
    add_convert(
        kn,
        "SequenceGroupBackgroundColor",
        PName::BackGroundColor,
        &[SName::GroupHeader],
    );
    add_con_font(kn, "SequenceGroup", &[SName::Group]);
    add_con_font(kn, "SequenceGroupHeader", &[SName::GroupHeader]);
    add_convert(
        kn,
        "SequenceBoxBorderColor",
        PName::LineColor,
        &[SName::Box],
    );
    add_convert(
        kn,
        "SequenceBoxBackgroundColor",
        PName::BackGroundColor,
        &[SName::Box],
    );
    add_convert(kn, "SequenceBoxFontColor", PName::FontColor, &[SName::Box]);
    add_convert(
        kn,
        "SequenceLifeLineBorderColor",
        PName::LineColor,
        &[SName::LifeLine],
    );
    add_convert(
        kn,
        "SequenceLifeLineBackgroundColor",
        PName::BackGroundColor,
        &[SName::ActivationBox],
    );
    add_con_font(kn, "sequenceDelay", &[SName::Delay]);
    add_convert(
        kn,
        "sequenceDelayBorderColor",
        PName::LineColor,
        &[SName::Delay],
    );
    add_convert(
        kn,
        "sequenceDividerBackgroundColor",
        PName::BackGroundColor,
        &[SName::Separator],
    );
    add_convert(
        kn,
        "sequenceDividerBorderColor",
        PName::LineColor,
        &[SName::Separator],
    );
    add_con_font(kn, "sequenceDivider", &[SName::Separator]);
    add_convert(
        kn,
        "sequenceDividerBorderThickness",
        PName::LineThickness,
        &[SName::Separator],
    );
    add_convert(
        kn,
        "SequenceMessageAlignment",
        PName::HorizontalAlignment,
        &[SName::Arrow],
    );
    add_con_font(kn, "note", &[SName::Note]);
    add_convert(
        kn,
        "noteBorderThickness",
        PName::LineThickness,
        &[SName::Note],
    );
    add_convert(kn, "noteBorderColor", PName::LineColor, &[SName::Note]);
    add_convert(
        kn,
        "noteBackgroundColor",
        PName::BackGroundColor,
        &[SName::Note],
    );
    add_convert(
        kn,
        "packageBackgroundColor",
        PName::BackGroundColor,
        &[SName::Group],
    );
    add_convert(kn, "packageBorderColor", PName::LineColor, &[SName::Group]);
    add_magic(kn, SName::Package);
    add_convert(
        kn,
        "PartitionBorderColor",
        PName::LineColor,
        &[SName::Composite],
    );
    add_convert(
        kn,
        "PartitionBackgroundColor",
        PName::BackGroundColor,
        &[SName::Composite],
    );
    add_con_font(kn, "Partition", &[SName::Composite]);
    add_convert(kn, "hyperlinkColor", PName::HyperLinkColor, &[SName::Root]);
    add_convert(
        kn,
        "activityStartColor",
        PName::BackGroundColor,
        &[SName::Circle, SName::Start],
    );
    add_convert(
        kn,
        "activityEndColor",
        PName::LineColor,
        &[SName::Circle, SName::End],
    );
    add_convert(
        kn,
        "activityStopColor",
        PName::LineColor,
        &[SName::Circle, SName::Stop],
    );
    add_convert(
        kn,
        "activityBarColor",
        PName::BackGroundColor,
        &[SName::ActivityBar],
    );
    add_convert(
        kn,
        "activityBorderColor",
        PName::LineColor,
        &[SName::Activity],
    );
    add_convert(
        kn,
        "activityBorderThickness",
        PName::LineThickness,
        &[SName::Activity],
    );
    add_convert(
        kn,
        "activityBackgroundColor",
        PName::BackGroundColor,
        &[SName::Activity],
    );
    add_con_font(kn, "activity", &[SName::Activity]);
    add_convert(
        kn,
        "activityDiamondBackgroundColor",
        PName::BackGroundColor,
        &[SName::Diamond],
    );
    add_convert(
        kn,
        "activityDiamondBorderColor",
        PName::LineColor,
        &[SName::Diamond],
    );
    add_con_font(kn, "activityDiamond", &[SName::Diamond]);
    add_con_font(kn, "arrow", &[SName::Arrow]);
    add_convert(kn, "arrowThickness", PName::LineThickness, &[SName::Arrow]);
    add_convert(kn, "arrowColor", PName::LineColor, &[SName::Arrow]);
    add_convert(kn, "arrowStyle", PName::LineStyle, &[SName::Arrow]);
    add_convert(kn, "arrowHeadColor", PName::HeadColor, &[SName::Arrow]);
    add_convert(
        kn,
        "defaulttextalignment",
        PName::HorizontalAlignment,
        &[SName::Root],
    );
    add_convert(kn, "defaultFontName", PName::FontName, &[SName::Root]);
    add_convert(kn, "defaultFontColor", PName::FontColor, &[SName::Root]);
    add_con_font(kn, "SwimlaneTitle", &[SName::Swimlane]);
    add_convert(
        kn,
        "SwimlaneTitleBackgroundColor",
        PName::BackGroundColor,
        &[SName::Swimlane],
    );
    add_convert(
        kn,
        "SwimlaneBorderColor",
        PName::LineColor,
        &[SName::Swimlane],
    );
    add_convert(
        kn,
        "SwimlaneBorderThickness",
        PName::LineThickness,
        &[SName::Swimlane],
    );
    add_convert(kn, "roundCorner", PName::RoundCorner, &[SName::Root]);
    add_convert(
        kn,
        "titleBorderThickness",
        PName::LineThickness,
        &[SName::Title],
    );
    add_convert(kn, "titleBorderColor", PName::LineColor, &[SName::Title]);
    add_convert(
        kn,
        "titleBackgroundColor",
        PName::BackGroundColor,
        &[SName::Title],
    );
    add_convert(
        kn,
        "titleBorderRoundCorner",
        PName::RoundCorner,
        &[SName::Title],
    );
    add_con_font(kn, "title", &[SName::Document, SName::Title]);
    add_convert(
        kn,
        "legendBorderThickness",
        PName::LineThickness,
        &[SName::Legend],
    );
    add_convert(kn, "legendBorderColor", PName::LineColor, &[SName::Legend]);
    add_convert(
        kn,
        "legendBackgroundColor",
        PName::BackGroundColor,
        &[SName::Legend],
    );
    add_convert(
        kn,
        "legendBorderRoundCorner",
        PName::RoundCorner,
        &[SName::Legend],
    );
    add_con_font(kn, "legend", &[SName::Legend]);
    add_convert(
        kn,
        "noteTextAlignment",
        PName::HorizontalAlignment,
        &[SName::Note],
    );
    add_convert(
        kn,
        "BackgroundColor",
        PName::BackGroundColor,
        &[SName::Document],
    );
    add_convert(
        kn,
        "classBackgroundColor",
        PName::BackGroundColor,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classBorderColor",
        PName::LineColor,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classFontSize",
        PName::FontSize,
        &[SName::Element, SName::Class, SName::Header],
    );
    add_convert(
        kn,
        "classFontStyle",
        PName::FontStyle,
        &[SName::Element, SName::Class, SName::Header],
    );
    add_convert(
        kn,
        "classFontColor",
        PName::FontColor,
        &[SName::Element, SName::Class, SName::Header],
    );
    add_convert(
        kn,
        "classFontName",
        PName::FontName,
        &[SName::Element, SName::Class, SName::Header],
    );
    add_convert(
        kn,
        "classStereotypeFontSize",
        PName::FontSize,
        &[SName::Element, SName::Class, SName::Stereotype],
    );
    add_convert(
        kn,
        "classStereotypeFontStyle",
        PName::FontStyle,
        &[SName::Element, SName::Class, SName::Stereotype],
    );
    add_convert(
        kn,
        "classStereotypeFontColor",
        PName::FontColor,
        &[SName::Element, SName::Class, SName::Stereotype],
    );
    add_convert(
        kn,
        "classStereotypeFontName",
        PName::FontName,
        &[SName::Element, SName::Class, SName::Stereotype],
    );
    add_convert(
        kn,
        "classAttributeFontSize",
        PName::FontSize,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classAttributeFontStyle",
        PName::FontStyle,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classAttributeFontColor",
        PName::FontColor,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classAttributeFontName",
        PName::FontName,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classBorderThickness",
        PName::LineThickness,
        &[SName::Element, SName::Class],
    );
    add_convert(
        kn,
        "classHeaderBackgroundColor",
        PName::BackGroundColor,
        &[SName::Element, SName::Class, SName::Header],
    );
    add_convert(
        kn,
        "objectBackgroundColor",
        PName::BackGroundColor,
        &[SName::Object],
    );
    add_convert(kn, "objectBorderColor", PName::LineColor, &[SName::Object]);
    add_con_font(kn, "object", &[SName::Object]);
    add_con_font(kn, "objectAttribute", &[SName::Object]);
    add_convert(
        kn,
        "objectBorderThickness",
        PName::LineThickness,
        &[SName::Object],
    );
    add_convert(
        kn,
        "stateBackgroundColor",
        PName::BackGroundColor,
        &[SName::State],
    );
    add_convert(kn, "stateBorderColor", PName::LineColor, &[SName::State]);
    add_con_font(kn, "state", &[SName::State]);
    add_con_font(kn, "stateAttribute", &[SName::State]);
    add_convert(
        kn,
        "stateBorderThickness",
        PName::LineThickness,
        &[SName::State],
    );
    add_magic(kn, SName::Agent);
    add_magic(kn, SName::Artifact);
    add_magic(kn, SName::Card);
    add_magic(kn, SName::Interface);
    add_magic(kn, SName::Cloud);
    add_magic(kn, SName::Component);
    add_magic(kn, SName::File);
    add_magic(kn, SName::Folder);
    add_magic(kn, SName::Frame);
    add_magic(kn, SName::Hexagon);
    add_magic(kn, SName::Node);
    add_magic(kn, SName::Person);
    add_magic(kn, SName::Queue);
    add_magic(kn, SName::Rectangle);
    add_magic(kn, SName::Stack);
    add_magic(kn, SName::Storage);
    add_magic(kn, SName::Usecase);
    add_magic(kn, SName::Map);
    add_magic(kn, SName::Archimate);
    add_magic(kn, SName::Hnote);
    add_magic(kn, SName::Rnote);
    add_convert(
        kn,
        "IconPrivateColor",
        PName::LineColor,
        &[SName::VisibilityIcon, SName::Private],
    );
    add_convert(
        kn,
        "IconPrivateBackgroundColor",
        PName::BackGroundColor,
        &[SName::VisibilityIcon, SName::Private],
    );
    add_convert(
        kn,
        "IconPackageColor",
        PName::LineColor,
        &[SName::VisibilityIcon, SName::Package],
    );
    add_convert(
        kn,
        "IconPackageBackgroundColor",
        PName::BackGroundColor,
        &[SName::VisibilityIcon, SName::Package],
    );
    add_convert(
        kn,
        "IconProtectedColor",
        PName::LineColor,
        &[SName::VisibilityIcon, SName::Protected],
    );
    add_convert(
        kn,
        "IconProtectedBackgroundColor",
        PName::BackGroundColor,
        &[SName::VisibilityIcon, SName::Protected],
    );
    add_convert(
        kn,
        "IconPublicColor",
        PName::LineColor,
        &[SName::VisibilityIcon, SName::Public],
    );
    add_convert(
        kn,
        "IconPublicBackgroundColor",
        PName::BackGroundColor,
        &[SName::VisibilityIcon, SName::Public],
    );
    add_convert(kn, "MinClassWidth", PName::MinimumWidth, &[]);
    add_convert(
        kn,
        "nodeStereotypeFontSize",
        PName::FontSize,
        &[SName::Node, SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontSize",
        PName::FontSize,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontStyle",
        PName::FontStyle,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontColor",
        PName::FontColor,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "sequenceStereotypeFontName",
        PName::FontName,
        &[SName::Stereotype],
    );
    add_convert(kn, "lifelineStrategy", PName::LineStyle, &[SName::LifeLine]);
    add_convert(kn, "wrapWidth", PName::MaximumWidth, &[SName::Element]);
    add_convert(
        kn,
        "HyperlinkUnderline",
        PName::HyperlinkUnderlineThickness,
        &[SName::Element],
    );
    add_convert(
        kn,
        "StereotypeAlignment",
        PName::HorizontalAlignment,
        &[SName::Stereotype],
    );
    add_convert(
        kn,
        "stereotypeABackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotAbstractClass],
    );
    add_convert(
        kn,
        "stereotypeABorderColor",
        PName::LineColor,
        &[SName::SpotAbstractClass],
    );
    add_convert(
        kn,
        "stereotypeCBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotClass],
    );
    add_convert(
        kn,
        "stereotypeCBorderColor",
        PName::LineColor,
        &[SName::SpotClass],
    );
    add_convert(
        kn,
        "stereotypeEBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotEnum],
    );
    add_convert(
        kn,
        "stereotypeEBorderColor",
        PName::LineColor,
        &[SName::SpotEnum],
    );
    add_convert(
        kn,
        "stereotypeIBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotInterface],
    );
    add_convert(
        kn,
        "stereotypeIBorderColor",
        PName::LineColor,
        &[SName::SpotInterface],
    );
    add_convert(
        kn,
        "stereotypeNBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotAnnotation],
    );
    add_convert(
        kn,
        "stereotypeNBorderColor",
        PName::LineColor,
        &[SName::SpotAnnotation],
    );
    add_convert(
        kn,
        "stereotypeRBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotRecord],
    );
    add_convert(
        kn,
        "stereotypeRBorderColor",
        PName::LineColor,
        &[SName::SpotRecord],
    );
    add_convert(
        kn,
        "stereotypeDBackgroundColor",
        PName::BackGroundColor,
        &[SName::SpotDataClass],
    );
    add_convert(
        kn,
        "stereotypeDBorderColor",
        PName::LineColor,
        &[SName::SpotDataClass],
    );
    k
});

/// Styles produced by one `skinparam key value` line.
pub struct FromSkinparamToStyle {
    styles: Vec<Style>,
    stereo: Option<String>,
    key: String,
}

impl FromSkinparamToStyle {
    pub fn new(key: &str) -> Self {
        if key.contains("<<") {
            let mut it = key.split(['<', '>']).filter(|s| !s.is_empty());
            let k = it.next().unwrap_or_default().to_string();
            let stereo = it.next().map(|s| s.trim().to_string());
            Self {
                styles: Vec::new(),
                stereo,
                key: k,
            }
        } else {
            Self {
                styles: Vec::new(),
                stereo: None,
                key: key.to_string(),
            }
        }
    }

    pub fn styles(&self) -> &[Style] {
        &self.styles
    }

    pub fn into_styles(self) -> Vec<Style> {
        self.styles
    }

    pub fn convert_now(&mut self, value: &str, builder: &mut StyleBuilder) {
        let mut value = value.to_string();
        if self.key.ends_with("shadowing") {
            if value.eq_ignore_ascii_case("false") {
                value = "0".into();
            } else if value.eq_ignore_ascii_case("true") {
                value = "3".into();
            }
        } else if self.key == "hyperlinkunderline" {
            if value.eq_ignore_ascii_case("false") {
                value = "0".into();
            }
            if value.eq_ignore_ascii_case("true") {
                value = "1".into();
            }
        }
        if value.eq_ignore_ascii_case("right:right") {
            value = "right".into();
        }
        if value.eq_ignore_ascii_case("dotted") {
            value = "1;3".into();
        }
        if value.eq_ignore_ascii_case("dashed") {
            value = "7;7".into();
        }
        let Some(datas) = KNOWLEDGE.get(&self.key.to_lowercase()) else {
            if self.key.eq_ignore_ascii_case("shadowing") {
                let v = shadowing_value(&value, builder);
                self.add_style(PName::Shadowing, v, &[SName::Root]);
            } else if self.key.eq_ignore_ascii_case("noteshadowing") {
                let v = shadowing_value(&value, builder);
                self.add_style(PName::Shadowing, v, &[SName::Root, SName::Note]);
            }
            return;
        };
        if is_complex(&value) {
            if value.contains(';') {
                if value.starts_with(';') {
                    value = format!(" {value}");
                }
                let mut it = value.split(';').filter(|s| !s.is_empty());
                let first = it.next().unwrap_or_default().to_string();
                let rest: Vec<String> = it.map(str::to_string).collect();
                for read in rest {
                    self.read_value(&read, datas, builder);
                }
                value = first;
            } else {
                self.read_value(&value, datas, builder);
                return;
            }
        }
        if value != " " {
            for d in datas {
                let v = regular(&value, builder);
                self.add_style(d.property, v, &d.style_names);
            }
        }
    }

    fn read_value(&mut self, read: &str, datas: &[Data], builder: &mut StyleBuilder) {
        let apply = |me: &mut Self, p: PName, v: &str, b: &mut StyleBuilder| {
            for d in datas {
                let val = regular(v, b);
                me.add_style(p, val, &d.style_names);
            }
        };
        if let Some(rest) = read.strip_prefix("text:") {
            let v2 = rest.split(':').next().unwrap_or_default();
            apply(self, PName::FontColor, v2, builder);
        } else if read.starts_with("line.dotted") {
            apply(self, PName::LineStyle, "1;3", builder);
        } else if read.starts_with("line.dashed") {
            apply(self, PName::LineStyle, "7;7", builder);
        } else if read.to_lowercase().contains("bold") {
            apply(self, PName::LineThickness, "2", builder);
        }
    }

    fn add_style(&mut self, property: PName, value: Value, names: &[SName]) {
        let mut sig = StyleQuery::of(names);
        let mut value = value;
        if let Some(stereo) = &self.stereo {
            let parts: Vec<&str> = stereo.split('&').collect();
            if let Value::Str {
                light,
                dark,
                specificity,
            } = value
            {
                value = Value::of(light, dark, specificity.with_stereotype_count(parts.len()));
            }
            for s in parts {
                sig = sig.with_stereotype(s);
            }
        }
        let mut map = BTreeMap::new();
        map.insert(property, value);
        self.styles.push(Style::new(sig, map));
    }
}

fn regular(value: &str, builder: &mut StyleBuilder) -> Value {
    Value::regular(value, Specificity::at_order(builder.next_int()))
}

fn shadowing_value(value: &str, builder: &mut StyleBuilder) -> Value {
    if value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("no") {
        regular("0", builder)
    } else if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes") {
        regular("3", builder)
    } else {
        regular(value, builder)
    }
}

fn is_complex(value: &str) -> bool {
    value.contains(';') || value.starts_with("text:")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(key: &str, value: &str) -> Vec<Style> {
        let mut b = StyleBuilder::new();
        let mut c = FromSkinparamToStyle::new(key);
        c.convert_now(value, &mut b);
        c.into_styles()
    }

    #[test]
    fn simple_conversion() {
        let s = convert("classbackgroundcolor", "red");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].value(PName::BackGroundColor).as_string(), "red");
        assert!(convert("unknownthing", "x").is_empty());
    }

    #[test]
    fn magic_and_font() {
        let s = convert("participantfontsize", "20");
        assert_eq!(s[0].value(PName::FontSize).as_int(), 20);
        let s = convert("actorbordercolor", "blue");
        assert_eq!(s[0].value(PName::LineColor).as_string(), "blue");
        assert_eq!(convert("packagebackgroundcolor", "red").len(), 2);
    }

    #[test]
    fn shadowing_and_special_values() {
        assert_eq!(
            convert("shadowing", "true")[0]
                .value(PName::Shadowing)
                .as_string(),
            "3"
        );
        assert_eq!(
            convert("shadowing", "false")[0]
                .value(PName::Shadowing)
                .as_string(),
            "0"
        );
        assert_eq!(
            convert("shadowing", "yes")[0]
                .value(PName::Shadowing)
                .as_string(),
            "3"
        );
        assert_eq!(
            convert("shadowing", "2")[0]
                .value(PName::Shadowing)
                .as_string(),
            "2"
        );
        assert_eq!(
            convert("noteshadowing", "no")[0]
                .value(PName::Shadowing)
                .as_string(),
            "0"
        );
        assert_eq!(
            convert("hyperlinkunderline", "false")[0]
                .value(PName::HyperlinkUnderlineThickness)
                .as_string(),
            "0"
        );
        assert_eq!(
            convert("hyperlinkunderline", "true")[0]
                .value(PName::HyperlinkUnderlineThickness)
                .as_string(),
            "1"
        );
        assert_eq!(
            convert("arrowstyle", "dotted")[0]
                .value(PName::LineStyle)
                .as_string(),
            "1"
        );
        assert_eq!(
            convert("arrowstyle", "dashed")[0]
                .value(PName::LineStyle)
                .as_string(),
            "7"
        );
        assert_eq!(
            convert("defaulttextalignment", "right:right")[0]
                .value(PName::HorizontalAlignment)
                .as_string(),
            "right"
        );
    }

    #[test]
    fn complex_values() {
        let s = convert("noteBorderColor", "red;text:blue;line.dotted;bold");
        let props: Vec<_> = s
            .iter()
            .flat_map(|s| {
                [
                    PName::LineColor,
                    PName::FontColor,
                    PName::LineStyle,
                    PName::LineThickness,
                ]
                .into_iter()
                .filter(|p| s.has_value(*p))
                .collect::<Vec<_>>()
            })
            .collect();
        assert!(props.contains(&PName::LineColor));
        assert!(props.contains(&PName::FontColor));
        assert!(props.contains(&PName::LineStyle));
        assert!(props.contains(&PName::LineThickness));
        let s = convert("noteBorderColor", "text:green");
        assert_eq!(s[0].value(PName::FontColor).as_string(), "green");
        let s = convert("noteBorderColor", "x;line.dashed");
        assert_eq!(s[0].value(PName::LineStyle).as_string(), "7;7");
        assert_eq!(convert("noteBorderColor", ";bold").len(), 1);
    }

    #[test]
    fn stereotypes() {
        let s = convert("classbackgroundcolor<<Foo&Bar>>", "red");
        assert!(s[0].query().has_stereotype());
        assert!(s[0]
            .value(PName::BackGroundColor)
            .specificity()
            .has_stereotype());
        let c = FromSkinparamToStyle::new("a<<b>>");
        assert!(c.styles().is_empty());
    }
}
