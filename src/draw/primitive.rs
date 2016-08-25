use super::{Shadable, Shader, Vertex, Color, Rect};

use cgmath::{Vector2};

use std::cell::{Cell, UnsafeCell};

pub struct ColorRect {
    pub color: Color,
    pub rect: Rect,
    num_updates: Cell<u64>,
    old_color: Cell<Color>,
    old_rect: Cell<Rect>,
    verts: UnsafeCell<[Vertex; 4]>
}

impl ColorRect {
    pub fn new(color: Color, rect: Rect) -> ColorRect {
        use std::mem;
        ColorRect {
            color: color,
            rect: rect,
            num_updates: Cell::new(0),
            old_color: Cell::new(color),
            old_rect: Cell::new(rect),
            verts: UnsafeCell::new(unsafe{ mem::zeroed() })
        }
    }
}

impl Shadable for ColorRect {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        // Yes, this is writing to potentially pointed-to data. However, the data being written isn't at
        // all different from the data that would have been in verts anyway, so we can get away with that.
        let verts = unsafe{ &mut *self.verts.get() };
        *verts = [
            Vertex::new(
                self.rect.upleft,
                Vector2::new(-SQRT_2, SQRT_2),
                self.color
            ),
            Vertex::new(
                self.rect.upright(),
                Vector2::new(SQRT_2, SQRT_2),
                self.color
            ),
            Vertex::new(
                self.rect.lowright,
                Vector2::new(SQRT_2, -SQRT_2),
                self.color
            ),
            Vertex::new(
                self.rect.lowleft(),
                Vector2::new(-SQRT_2, -SQRT_2),
                self.color
            )
        ];

        const INDICES: &'static [u16] = 
            &[0, 1, 2,
              2, 3, 0];

        Shader::Verts {
            verts: unsafe{ &*self.verts.get() },
            indices: INDICES
        }
    }

    fn num_updates(&self) -> u64 {
        if self.old_color.get() != self.color ||
           self.old_rect.get() != self.rect {
            self.num_updates.set(self.num_updates.get() + 1);
            self.old_color.set(self.color);
            self.old_rect.set(self.rect);
        }

        self.num_updates.get()
    }
}

impl<'b> Shadable for &'b ColorRect {
    type Composite = ();
    fn shader_data<'a>(&'a self) -> Shader<'a, ()> {
        (*self).shader_data()
    }

    fn num_updates(&self) -> u64 {
        (*self).num_updates()
    }
}

const SQRT_2: f32 = 0.70710678118;