//! Title and stereotype of a package (`ClusterHeader`).

use crate::abel::{Entity, GroupType};
use crate::cucadiagram::CucaDiagram;
use crate::decoration::usymbol::USymbol;
use crate::klimt::creole::display::Display;
use crate::klimt::font::{Dimension, StringBounder};
use crate::klimt::geom::{HorizontalAlignment, VerticalAlignment};
use crate::klimt::shape::{with_margin, EmptyBlock, HorizontalBlock, TextBlock, VerticalBlock};
use crate::skin::SkinParam;
use crate::style::{queries, Margins, PName, SName, Style, StyleQuery};
use std::rc::Rc;

pub struct ClusterHeader {
    pub title: Rc<dyn TextBlock>,
    pub stereo: Rc<dyn TextBlock>,
    pub title_alignment: HorizontalAlignment,
    pub width: i32,
    pub height: i32,
}

fn empty() -> Rc<dyn TextBlock> {
    Rc::new(EmptyBlock {
        width: 0.0,
        height: 0.0,
    })
}

/// `Cluster.getDefaultStyleDefinition`.
pub fn default_style_query(
    diagram_style: SName,
    symbol: Option<&Rc<dyn USymbol>>,
    group_type: Option<GroupType>,
) -> StyleQuery {
    if diagram_style == SName::StateDiagram {
        return queries::STATEDIAG_STATE_GROUP.clone();
    }
    if let Some(sym) = symbol {
        return queries::GROUP.add(diagram_style).add_all(&sym.snames());
    }
    if group_type == Some(GroupType::Package) {
        return queries::PACKAGE_GROUP.add(diagram_style);
    }
    queries::GROUP.add(diagram_style)
}

fn title_style(g: &Entity, skin: &SkinParam, diagram_style: SName) -> Style {
    let signature = if g.group_type() == Some(GroupType::State) {
        queries::STATEDIAG_STATE_NAME.clone()
    } else if let Some(sym) = g.usymbol() {
        queries::COMPOSITE_TITLE
            .add(diagram_style)
            .add_all(&sym.snames())
    } else if g.group_type() == Some(GroupType::Package) {
        queries::PACKAGE_TITLE.add(diagram_style)
    } else {
        queries::COMPOSITE_TITLE.add(diagram_style)
    };
    skin.style_builder()
        .merged_style(&signature.with_stereotype_obj(g.stereotype().as_ref()))
        .expect("cluster title style")
}

impl ClusterHeader {
    pub fn new(
        g: &Entity,
        cuca: &CucaDiagram,
        skin: &SkinParam,
        diagram_style: SName,
        sb: &dyn StringBounder,
    ) -> ClusterHeader {
        let skin_simple = skin.skin_simple();
        let style = title_style(g, skin, diagram_style);
        let alignment = style.horizontal_alignment();
        let mut title: Rc<dyn TextBlock> = {
            let label = g.display();
            if label.is_null() {
                empty()
            } else {
                let fc = style.font_configuration(Some(&g.colors()));
                label.create_simple_block(&fc, alignment, &skin_simple)
            }
        };
        if let Some(m) = g.get().visibility {
            let st = skin
                .style_builder()
                .merged_style(&m.style_query())
                .expect("icon style");
            let fore = st.value(PName::LineColor).as_color();
            let back = if m.is_field() {
                None
            } else {
                Some(st.value(PName::BackGroundColor).as_color())
            };
            let icon = with_margin(
                m.u_block(skin.class_attribute_icon_size(), fore, back, false),
                Margins::new(4.0, 0.0, 0.0, 0.0),
            );
            title = Rc::new(HorizontalBlock {
                blocks: vec![icon, title],
                alignment: VerticalAlignment::Center,
            });
        }
        let stereo = Self::stereo_block(g, cuca, skin, diagram_style, alignment);
        let merged = VerticalBlock {
            blocks: vec![stereo.clone(), title.clone()],
            alignment,
        };
        let dim_label = merged.calculate_dimension(sb);
        let (mut width, mut height) = (0, 0);
        if dim_label.width > 0.0 {
            let (supp_w, supp_h) = match g.usymbol() {
                Some(u) => (
                    u.supp_width_because_of_shape(),
                    u.supp_height_because_of_shape(),
                ),
                None => (0, 0),
            };
            width = dim_label.width as i32 + supp_w;
            height = (dim_label.height + f64::from(supp_h)) as i32;
        }
        ClusterHeader {
            title,
            stereo,
            title_alignment: alignment,
            width,
            height,
        }
    }

    fn stereo_block(
        g: &Entity,
        cuca: &CucaDiagram,
        skin: &SkinParam,
        diagram_style: SName,
        alignment: HorizontalAlignment,
    ) -> Rc<dyn TextBlock> {
        let Some(stereotype) = g.stereotype() else {
            return empty();
        };
        if diagram_style == SName::StateDiagram && g.usymbol().is_none() {
            return empty();
        }
        let Some(visible) = cuca.visible_stereotype_labels(g) else {
            return empty();
        };
        if visible.is_empty() {
            return empty();
        }
        let q = default_style_query(diagram_style, g.usymbol().as_ref(), g.group_type())
            .for_stereotype_itself(Some(&stereotype));
        let style = skin.style_builder().merged_style(&q).expect("stereo style");
        Display::create(&visible).create_simple_block(
            &style.font_configuration(None),
            alignment,
            &skin.skin_simple(),
        )
    }

    pub fn title_dimension(&self, sb: &dyn StringBounder) -> Dimension {
        self.title.calculate_dimension(sb)
    }
}
