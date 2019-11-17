use glium::glutin::dpi::LogicalSize;

use glium::{glutin, Surface};

fn main() {
    // 1. The **winit::EventsLoop** for handling events.
    let mut events_loop = glium::glutin::EventsLoop::new();
    // 2. Parameters for building the Window.
    let wb = glium::glutin::WindowBuilder::new()
        .with_dimensions(LogicalSize::new(1280.0, 720.0))
        .with_title("Hello world");
    // 3. Parameters for building the OpenGL context.
    let cb = glium::glutin::ContextBuilder::new();
    // 4. Build the Display with the given window and OpenGL context parameters and register the
    //    window with the events_loop.
    let display = glium::Display::new(wb, cb, &events_loop).unwrap();

    let (vertex_buffer, index_buffer) = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 3],
        }

        glium::implement_vertex!(Vertex, position);

        fn gen_quads(
            interval: ((f32, f32), (f32, f32)),
            segments: (usize, usize),
            generator: impl Fn(f32, f32) -> f32,
        ) -> (Vec<Vertex>, Vec<u32>) {
            let mut vertices = Vec::new();
            let mut indices = Vec::new();

            let x_interval = interval.0;
            let y_interval = interval.1;

            let dx = (x_interval.0 - x_interval.1).abs() / segments.0 as f32;
            let dy = (y_interval.0 - y_interval.1).abs() / segments.1 as f32;

            for y in 0..segments.1 {
                for x in 0..segments.0 {
                    let x1 = x_interval.0 + dx * x as f32;
                    let x2 = x_interval.0 + dx * (x + 1) as f32;
                    let y1 = y_interval.0 + dy * y as f32;
                    let y2 = y_interval.0 + dy * (y + 1) as f32;

                    let vertex_at = |x, y| Vertex {
                        position: [x, y, generator(x, y)],
                    };

                    // tri 1
                    indices.push(vertices.len() as u32 + 0);
                    indices.push(vertices.len() as u32 + 1);
                    indices.push(vertices.len() as u32 + 3);
                    // tri 2
                    indices.push(vertices.len() as u32 + 1);
                    indices.push(vertices.len() as u32 + 2);
                    indices.push(vertices.len() as u32 + 3);

                    vertices.push(vertex_at(x1, y1));
                    vertices.push(vertex_at(x2, y1));
                    vertices.push(vertex_at(x2, y2));
                    vertices.push(vertex_at(x1, y2));
                }
            }

            (vertices, indices)
        }

        let (vertices, indices) = gen_quads(
            ((-5.0, 5.0), (-5.0, 5.0)),
            (1000, 1000),
            |x: f32, y: f32| {
                // x * (-(x*x + y*y)).exp()
                // (10.0 * (x*x+y*y)).sin() / 10.0
                // (x*y).signum() * (1.0 - (x*9.0) * (x*9.0) + (y*9.0)*(y*9.0)).signum() / 9.0
                1.0 - (x + y).abs() - (y - x).abs()
                // (x*x + y*y).powf(0.5)
                // x.sin() * y.cos()
                // (0.4*0.4 - (0.6 - (x*x+y*y).powf(0.5)).powf(2.0)).powf(0.5)
                // (1.0 - (-x - 0.51 + (y*2.0).abs()).signum()) / 3.0 * ((0.5-x).signum()+1.0)/3.0
                // (x - 1.0 + (y * 2.0).abs()).signum() / 3.0 + (x - 0.5 + (y * 2.0).abs()).signum() / 3.0
            },
        );

        (
            glium::VertexBuffer::new(&display, &vertices).unwrap(),
            glium::IndexBuffer::new(
                &display,
                glium::index::PrimitiveType::TrianglesList,
                &indices,
            )
            .unwrap(),
        )
    };

    let vs_src = include_str!("../res/shaders/surface.vert");
    let fs_src = include_str!("../res/shaders/surface.frag");
    let wf_src = include_str!("../res/shaders/wireframe.frag");

    let pvt_src = include_str!("../res/shaders/text.vert");
    let pvf_src = include_str!("../res/shaders/text.frag");

    let program = glium::Program::from_source(&display, vs_src, fs_src, None).unwrap();
    let program_wf = glium::Program::from_source(&display, vs_src, wf_src, None).unwrap();
    let ptext = glium::Program::from_source(&display, pvt_src, pvf_src, None).unwrap();

    let eye = nalgebra::Point3::new(-12.0, -12.0, 20.0) / 2.0;
    let target = nalgebra::Point3::new(0.0, 0.0, 0.0);

    let proj = nalgebra::Perspective3::new(16.0 / 9.0, 3.14 / 4.0, 0.01, 100.0);
    let view = nalgebra::geometry::Isometry3::look_at_rh(&eye, &target, &nalgebra::Vector3::z());

    let mvp_mat = proj.as_matrix() * view.to_homogeneous();
    let mvp: [[f32; 4]; 4] = *mvp_mat.as_ref();

    // fonts
    use rusttype::*;

    let font_data = include_bytes!("../res/fonts/arial.ttf");
    let font = Font::from_bytes(font_data as &[u8]).unwrap();


    use glium::{
        texture::{ClientFormat, MipmapsOption, RawImage2d, UncompressedFloatFormat},
        Texture2d,
    };
    use std::borrow::Cow;
    use std::collections::HashMap;

    #[derive(Clone, Copy, Debug)]
    struct CachedGlyph {
        pub symbol: char,
        pub uv_min: (f32, f32),
        pub uv_max: (f32, f32),
    }

    #[derive(Debug)]
    struct GlyphCache {
        pub texture: Texture2d,
        current_offset: (u32, u32),
        current_line_height: u32,
        glyphs: HashMap<char, CachedGlyph>,
        font: rusttype::Font<'static>,
        scale: rusttype::Scale,
    }

    #[derive(Clone, Copy, Debug)]
    enum CacheError {
        NotEnoughSpace,
    }

    impl GlyphCache {
        pub fn empty<F: ?Sized + glium::backend::Facade>(
            facade: &F,
            font: rusttype::Font<'static>,
            scale: rusttype::Scale,
            width: u32,
            height: u32,
        ) -> Self {
            GlyphCache {
                current_offset: (0, 0),
                glyphs: HashMap::new(),
                font,
                scale,
                current_line_height: 0,
                texture: Texture2d::with_format(
                    facade,
                    RawImage2d {
                        data: Cow::Owned(vec![128u8; (width * height) as usize]),
                        width,
                        height,
                        format: ClientFormat::U8,
                    },
                    UncompressedFloatFormat::U8,
                    MipmapsOption::NoMipmap,
                )
                .expect("Oopsie woopsie, I did a fucky wucky"),
            }
        }

        pub fn width(&self) -> u32 {
            self.texture.width()
        }

        pub fn height(&self) -> u32 {
            self.texture.height()
        }

        pub fn cache(&mut self, symbol: char) -> Result<CachedGlyph, CacheError> {
            // if we already cached this one, just return it
            if let Some(glyph) = self.glyphs.get(&symbol) {
                return Ok(*glyph);
            }

            let glyph = self
                .font
                .glyph(symbol)
                .scaled(self.scale)
                .positioned(point(0.0, 0.0));

            let (gwidth, gheight) = (
                glyph.pixel_bounding_box().unwrap().width() as u32,
                glyph.pixel_bounding_box().unwrap().height() as u32,
            );

            // check if we fit on the current line
            if self.current_offset.0 + gwidth >= self.width() {
                // check if we are out of space entirely
                if self.current_offset.1 + self.current_line_height >= self.height() {
                    return Err(CacheError::NotEnoughSpace);
                }
                // move to another line
                self.current_offset.0 = 0;
                self.current_offset.1 += self.current_line_height;
                self.current_line_height = 0;
            }

            let mut pixels: Vec<u8> = vec![0; (gwidth * gheight) as usize];
            glyph.draw(|x, y, v| {
                pixels[(x + (gheight - (y + 1)) * gwidth) as usize] = (v * 255.0) as u8;
            });

            // update the texture
            self.texture.write(
                glium::Rect {
                    left: self.current_offset.0,
                    bottom: self.height() - self.current_offset.1 - gheight,
                    width: gwidth,
                    height: gheight,
                },
                RawImage2d {
                    data: Cow::Borrowed(&pixels),
                    width: gwidth,
                    height: gheight,
                    format: ClientFormat::U8,
                },
            );

            let cached = CachedGlyph {
                uv_min: (self.current_offset.0 as f32 / self.width() as f32, self.current_offset.1 as f32 / self.height() as f32),
                uv_max: ((self.current_offset.0 + gwidth) as f32 / self.width() as f32, (self.current_offset.1 + gheight) as f32 / self.height() as f32),
                symbol,
            };

            self.glyphs.insert(symbol, cached);

            // when we are done update the offset
            self.current_offset.0 += gwidth;
            self.current_line_height = u32::max(self.current_line_height, gheight);
            Ok(cached)
        }
    }

    let mut gcache = GlyphCache::empty(&display, font.clone(), Scale::uniform(48.0), 1024, 1024);

    for c in ('A' as u8)..=('Z' as u8) {
        gcache.cache(c as char).unwrap();
    }
    for c in ('a' as u8)..=('z' as u8) {
        gcache.cache(c as char).unwrap();
    }

    // println!("{:?}, {}, {}", gcache, 'a' as u8, 'Z' as u8);

    let text = "ExampleText";
    let layout = gcache
        .font
        .layout(text, Scale::uniform(48.0), point(0.0, 0.0))
        .collect::<Vec<_>>();

    let (vtext, itext) = {
        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 2],
            uv: [f32; 2],
        }

        glium::implement_vertex!(Vertex, position, uv);

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (i, g) in layout.iter().enumerate() {
            dbg!(text.chars().nth(i));
            let cached = gcache.cache(text.chars().nth(i).unwrap()).unwrap();
            let bb = g.pixel_bounding_box().unwrap();

            indices.push(vertices.len() as u32 + 0);
            indices.push(vertices.len() as u32 + 1);
            indices.push(vertices.len() as u32 + 3);
            // tri 2
            indices.push(vertices.len() as u32 + 1);
            indices.push(vertices.len() as u32 + 2);
            indices.push(vertices.len() as u32 + 3);

            let vertex = |x, y, u, v| Vertex {
                position: [x, y],
                uv: [u, v],
            };

            vertices.push(vertex(bb.min.x as f32, bb.min.y as f32, cached.uv_min.0, cached.uv_min.1));
            vertices.push(vertex(bb.max.x as f32, bb.min.y as f32, cached.uv_max.0, cached.uv_min.1));
            vertices.push(vertex(bb.max.x as f32, bb.max.y as f32, cached.uv_max.0, cached.uv_max.1));
            vertices.push(vertex(bb.min.x as f32, bb.max.y as f32, cached.uv_min.0, cached.uv_max.1));
        }

        (
            glium::VertexBuffer::new(&display, &vertices).unwrap(),
            glium::IndexBuffer::new(
                &display,
                glium::index::PrimitiveType::TrianglesList,
                &indices,
            )
            .unwrap(),
        )
    };

    let uniforms = glium::uniform! {
        mvp: mvp,
        glyph_atlas: &gcache.texture
    };

    use nalgebra::{Vector3, Vector2, Isometry3, Isometry2, Rotation2, Rotation3, Translation2, Translation3};
    let text_model: [[f32; 4]; 4] = *Translation3::new(0.0, 300.0, 0.0).to_homogeneous().as_ref();

    let orth_proj = nalgebra::Orthographic3::new(0.0, 1280.0, 720.0, 0.0, 0.001, 100.0);
    let orth_proj: [[f32; 4]; 4] = *orth_proj.as_matrix().as_ref();

    let mut should_close = false;
    while !should_close {
        let mut target = display.draw();

        target.clear_color_and_depth((0.0, 0.0, 0.0, 1.0), 1.0);
        target
            .draw(
                &vertex_buffer,
                &index_buffer,
                &program,
                &uniforms,
                &glium::DrawParameters {
                    depth: glium::Depth {
                        test: glium::draw_parameters::DepthTest::IfLess,
                        write: true,
                        ..Default::default()
                    },
                    ..glium::DrawParameters::default()
                },
            )
            .unwrap();

        target
            .draw(
                &vtext,
                &itext,
                &ptext,
                &glium::uniform! {
                    model: text_model,
                    proj: orth_proj,
                    glyph_atlas: &gcache.texture
                },
                &glium::DrawParameters {
                    blend: glium::draw_parameters::Blend::alpha_blending(),
                    ..glium::DrawParameters::default()
                },
            )
            .unwrap();

        // target
        //     .draw(
        //         &vertex_buffer,
        //         &index_buffer,
        //         &program_wf,
        //         &uniforms,
        //         &glium::DrawParameters {
        //             depth: glium::Depth {
        //                 test: glium::draw_parameters::DepthTest::IfLessOrEqual,
        //                 write: false,
        //                 ..Default::default()
        //             },
        //             polygon_mode: glium::draw_parameters::PolygonMode::Line,
        //             blend: glium::draw_parameters::Blend::alpha_blending(),
        //             ..glium::DrawParameters::default()
        //         },
        //     )
        //     .unwrap();

        target.finish().unwrap();
    }
}
