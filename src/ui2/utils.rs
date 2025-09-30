use ggez::{
    Context,
    graphics::{Canvas, Color, DrawMode, DrawParam, FillOptions, Mesh, Rect, Text, Transform},
};

pub fn draw_fps(ctx: &Context, canvas: &mut Canvas) {
    let height = ctx.gfx.drawable_size().1;
    let offset = height * 0.005;

    let mut text = Text::new(format!("FPS: {:.0}", ctx.time.fps()));
    text.set_scale(height * 0.03);

    let text_size = text.measure(ctx).unwrap();
    let rectangle = Rect::new(offset, offset, text_size.x, text_size.y);
    let mesh = Mesh::new_rectangle(
        ctx,
        DrawMode::Fill(FillOptions::default()),
        rectangle,
        Color::BLACK,
    )
    .unwrap();

    canvas.draw(&mesh, DrawParam::default());

    canvas.draw(
        &text,
        DrawParam {
            transform: Transform::Values {
                dest: [offset; 2].into(),
                rotation: 0.0,
                scale: [1.0; 2].into(),
                offset: [0.0; 2].into(),
            },
            ..Default::default()
        },
    );
}
