use crate::core::{Term, SymbolTable};
use crate::reasoning::rules::RuleEngine;
use super::dsl::{Grid, Object, connected_components, unique_colors, grid_dimensions,
    is_above, is_below, is_left_of, is_right_of, is_adjacent, is_inside,
    is_symmetric_h, is_symmetric_v, detect_period_h, detect_period_v};

pub struct GridReasoner {
    syms: SymbolTable,
    // Cached symbol IDs
    pub color_sym: u32,
    pub object_sym: u32,
    pub above_sym: u32,
    pub below_sym: u32,
    pub left_of_sym: u32,
    pub right_of_sym: u32,
    pub adjacent_sym: u32,
    pub inside_sym: u32,
    pub size_sym: u32,
    pub same_color_sym: u32,
    pub symmetric_h_sym: u32,
    pub symmetric_v_sym: u32,
    pub periodic_h_sym: u32,
    pub periodic_v_sym: u32,
    pub grid_width_sym: u32,
    pub grid_height_sym: u32,
    pub num_objects_sym: u32,
    pub num_colors_sym: u32,
    pub bbox_sym: u32,
}

impl GridReasoner {
    pub fn new() -> Self {
        let mut syms = SymbolTable::new();
        Self {
            color_sym: syms.intern("color"),
            object_sym: syms.intern("object"),
            above_sym: syms.intern("above"),
            below_sym: syms.intern("below"),
            left_of_sym: syms.intern("left_of"),
            right_of_sym: syms.intern("right_of"),
            adjacent_sym: syms.intern("adjacent"),
            inside_sym: syms.intern("inside"),
            size_sym: syms.intern("size"),
            same_color_sym: syms.intern("same_color"),
            symmetric_h_sym: syms.intern("symmetric_h"),
            symmetric_v_sym: syms.intern("symmetric_v"),
            periodic_h_sym: syms.intern("periodic_h"),
            periodic_v_sym: syms.intern("periodic_v"),
            grid_width_sym: syms.intern("grid_width"),
            grid_height_sym: syms.intern("grid_height"),
            num_objects_sym: syms.intern("num_objects"),
            num_colors_sym: syms.intern("num_colors"),
            bbox_sym: syms.intern("bbox"),
            syms,
        }
    }

    pub fn syms(&self) -> &SymbolTable {
        &self.syms
    }

    pub fn syms_mut(&mut self) -> &mut SymbolTable {
        &mut self.syms
    }

    pub fn analyze_grid(&self, grid: &Grid, engine: &mut RuleEngine) -> Vec<Object> {
        let objects = connected_components(grid, true);
        let colors = unique_colors(grid);
        let (rows, cols) = grid_dimensions(grid);

        // Grid properties
        engine.add_fact(Term::compound(self.grid_height_sym, vec![Term::int(rows as i64)]));
        engine.add_fact(Term::compound(self.grid_width_sym, vec![Term::int(cols as i64)]));
        engine.add_fact(Term::compound(self.num_objects_sym, vec![Term::int(objects.len() as i64)]));
        engine.add_fact(Term::compound(self.num_colors_sym, vec![Term::int(colors.len() as i64)]));

        // Symmetry
        if is_symmetric_h(grid) {
            engine.add_fact(Term::compound(self.symmetric_h_sym, vec![]));
        }
        if is_symmetric_v(grid) {
            engine.add_fact(Term::compound(self.symmetric_v_sym, vec![]));
        }
        if let Some(p) = detect_period_h(grid) {
            engine.add_fact(Term::compound(self.periodic_h_sym, vec![Term::int(p as i64)]));
        }
        if let Some(p) = detect_period_v(grid) {
            engine.add_fact(Term::compound(self.periodic_v_sym, vec![Term::int(p as i64)]));
        }

        // Object facts
        for (i, obj) in objects.iter().enumerate() {
            let id = Term::int(i as i64);

            // object(Id, Color, Area)
            engine.add_fact(Term::compound(self.object_sym, vec![
                id.clone(), Term::int(obj.color as i64), Term::int(obj.area() as i64),
            ]));

            // bbox(Id, MinR, MinC, H, W)
            let (mr, mc, h, w) = obj.bounding_box();
            engine.add_fact(Term::compound(self.bbox_sym, vec![
                id.clone(),
                Term::int(mr as i64), Term::int(mc as i64),
                Term::int(h as i64), Term::int(w as i64),
            ]));

            // color(Id, Color)
            engine.add_fact(Term::compound(self.color_sym, vec![
                id.clone(), Term::int(obj.color as i64),
            ]));

            // size(Id, Area)
            engine.add_fact(Term::compound(self.size_sym, vec![
                id, Term::int(obj.area() as i64),
            ]));
        }

        // Spatial relations
        for i in 0..objects.len() {
            for j in 0..objects.len() {
                if i == j { continue; }
                let oi = Term::int(i as i64);
                let oj = Term::int(j as i64);

                if is_above(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.above_sym, vec![oi.clone(), oj.clone()]));
                }
                if is_below(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.below_sym, vec![oi.clone(), oj.clone()]));
                }
                if is_left_of(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.left_of_sym, vec![oi.clone(), oj.clone()]));
                }
                if is_right_of(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.right_of_sym, vec![oi.clone(), oj.clone()]));
                }
                if is_adjacent(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.adjacent_sym, vec![oi.clone(), oj.clone()]));
                }
                if is_inside(&objects[i], &objects[j]) {
                    engine.add_fact(Term::compound(self.inside_sym, vec![oi.clone(), oj.clone()]));
                }
                if objects[i].color == objects[j].color {
                    engine.add_fact(Term::compound(self.same_color_sym, vec![oi, oj]));
                }
            }
        }

        objects
    }

    pub fn add_reasoning_rules(&self, _engine: &mut RuleEngine) {
        // Extensible: add derived rules for spatial reasoning
        // e.g. horizontally_aligned(A,B) :- left_of(A,B).
        // e.g. vertically_aligned(A,B) :- above(A,B).
    }
}
