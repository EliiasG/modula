use std::collections::BTreeMap;

use super::{AtlasLayout, AtlasLayouter, AtlasLayouterOutput, MaxAtlasSize, SubTexture};
use rectangle_pack::{
    contains_smallest_box, volume_heuristic, GroupedRectsToPlace, RectToInsert, RectanglePackError,
    TargetBin,
};

/// Default [AtlasLayouter], uses rectangle-pack
pub struct DefaultLayouter;

impl AtlasLayouter for DefaultLayouter {
    type Error = RectanglePackError;

    fn layout(
        sizes: Vec<(u32, u32)>,
        max_atlas_size: MaxAtlasSize,
    ) -> Result<AtlasLayouterOutput, RectanglePackError> {
        let mut rects = GroupedRectsToPlace::new();
        for (i, s) in sizes.iter().enumerate() {
            rects.push_rect(i, None, RectToInsert::new(s.0, s.1, 1));
        }
        let res = modula_utils::binsearch(
            |i| attempt(i as u32, 1, 1, &rects),
            1..max_atlas_size.max_width_hight as i32 + 1,
        );
        if res.is_ok() {
            return res;
        }
        modula_utils::binsearch_upwards(
            |i| {
                attempt(
                    max_atlas_size.max_width_hight,
                    max_atlas_size.max_layers,
                    i as u32,
                    &rects,
                )
            },
            1,
        )
    }
}

fn attempt(
    wh: u32,
    max_depth: u32,
    layers: u32,
    rects: &GroupedRectsToPlace<usize>,
) -> Result<AtlasLayouterOutput, RectanglePackError> {
    let mut bins = BTreeMap::new();

    for i in 0..layers {
        bins.insert(i, TargetBin::new(wh, wh, 1));
    }
    let packing =
        rectangle_pack::pack_rects(&rects, &mut bins, &volume_heuristic, &contains_smallest_box)?;
    let res = packing.packed_locations();

    let mut layout = vec![
        SubTexture {
            layer: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0
        };
        res.len()
    ];
    for (idx, (layer, location)) in res.iter() {
        layout[*idx] = SubTexture {
            layer: *layer,
            x: location.x(),
            y: location.y(),
            width: location.width(),
            height: location.height(),
        };
    }

    let atlas_count = layers.div_ceil(max_depth);
    let mut atlases = Vec::with_capacity(atlas_count as usize);
    for i in 0..atlas_count {
        atlases.push((
            (
                wh,
                wh,
                if i == atlas_count - 1 {
                    layers % max_depth
                } else {
                    max_depth
                },
            ),
            AtlasLayout(Vec::new()),
        ));
    }

    let mut entry_map = Vec::with_capacity(layout.len());
    for mut tex in layout {
        let atlas_idx = (tex.layer / max_depth) as usize;
        tex.layer %= max_depth;
        atlases[atlas_idx].1 .0.push(tex);
        entry_map.push((atlas_idx, atlases[atlas_idx].1 .0.len() - 1));
    }

    Ok(AtlasLayouterOutput { entry_map, atlases })
}
