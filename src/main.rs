use nalgebra_glm::{Vec3, Mat4, look_at, perspective};
use minifb::{Key, KeyRepeat, Window, WindowOptions};
use std::time::Duration;
use std::f32::consts::PI;
use image::{open, DynamicImage, GenericImageView};
use rand::Rng;


mod framebuffer;
mod triangle;
mod vertex;
mod obj;
mod color;
mod fragment;
mod shaders;
mod camera;

use framebuffer::Framebuffer;
use vertex::Vertex;
use obj::Obj;
use camera::Camera;
use triangle::triangle;
use shaders::{vertex_shader, fragment_shader};
use fastnoise_lite::{FastNoiseLite, NoiseType};



pub struct Uniforms {
    model_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    viewport_matrix: Mat4,
    time: u32,
    noise: FastNoiseLite,
    shader_mode: u8
}

fn create_noise() -> FastNoiseLite {
    create_cloud_noise()
}

fn create_cloud_noise() -> FastNoiseLite {
    let mut noise = FastNoiseLite::with_seed(1337);
    noise.set_noise_type(Some(NoiseType::OpenSimplex2));
    noise
}

fn create_model_matrix(translation: Vec3, scale: f32, rotation: Vec3) -> Mat4 {
    let (sin_x, cos_x) = rotation.x.sin_cos();
    let (sin_y, cos_y) = rotation.y.sin_cos();
    let (sin_z, cos_z) = rotation.z.sin_cos();

    let rotation_matrix_x = Mat4::new(
        1.0,  0.0,    0.0,   0.0,
        0.0,  cos_x, -sin_x, 0.0,
        0.0,  sin_x,  cos_x, 0.0,
        0.0,  0.0,    0.0,   1.0,
    );

    let rotation_matrix_y = Mat4::new(
        cos_y,  0.0,  sin_y, 0.0,
        0.0,    1.0,  0.0,   0.0,
        -sin_y, 0.0,  cos_y, 0.0,
        0.0,    0.0,  0.0,   1.0,
    );

    let rotation_matrix_z = Mat4::new(
        cos_z, -sin_z, 0.0, 0.0,
        sin_z,  cos_z, 0.0, 0.0,
        0.0,    0.0,  1.0, 0.0,
        0.0,    0.0,  0.0, 1.0,
    );

    let rotation_matrix = rotation_matrix_z * rotation_matrix_y * rotation_matrix_x;

    let transform_matrix = Mat4::new(
        scale, 0.0,   0.0,   translation.x,
        0.0,   scale, 0.0,   translation.y,
        0.0,   0.0,   scale, translation.z,
        0.0,   0.0,   0.0,   1.0,
    );

    transform_matrix * rotation_matrix
}


fn create_view_matrix(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    look_at(&eye, &center, &up)
}

fn create_perspective_matrix(window_width: f32, window_height: f32) -> Mat4 {
    let fov = 45.0 * PI / 180.0;
    let aspect_ratio = window_width / window_height;
    let near = 0.1;
    let far = 1000.0;

    perspective(fov, aspect_ratio, near, far)
}

fn create_viewport_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::new(
        width / 2.0, 0.0, 0.0, width / 2.0,
        0.0, -height / 2.0, 0.0, height / 2.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0
    )
}

fn render(framebuffer: &mut Framebuffer, uniforms: &Uniforms, vertex_array: &[Vertex], image: &DynamicImage, image_width: u32, image_height: u32) {
    // Renderizar la imagen panorámica de fondo
    // Iteramos por todos los píxeles de la ventana y proyectamos la imagen panorámica sobre el fondo
    for y in 0..framebuffer.height {
        for x in 0..framebuffer.width {
            // Convertimos las coordenadas de la ventana a ángulos esféricos
            let x_angle = (x as f32 / framebuffer.width as f32) * 360.0 - 180.0; // Mapeo de 0 a 360 -> -180 a 180
            let y_angle = (y as f32 / framebuffer.height as f32) * 180.0 - 90.0; // Mapeo de 0 a 180 -> -90 a 90

            // Proyectamos estos ángulos a coordenadas de la imagen panorámica
            let (x_pixel, y_pixel) = project_to_image(x_angle, y_angle, image_width, image_height);

            // Obtenemos el color del píxel correspondiente en la imagen panorámica
            let pixel = image.get_pixel(x_pixel, y_pixel);
            let color = (pixel[0] as u32) | ((pixel[1] as u32) << 8) | ((pixel[2] as u32) << 16);

            // Establecemos el color de fondo en el framebuffer
            framebuffer.set_current_color(color);
            framebuffer.point(x as usize, y as usize, 1.0);
        }
    }

    // Ahora renderizamos los objetos 3D, como la esfera, sobre el fondo de la imagen panorámica

    // Transforma los vértices con el shader de vértices (usando las matrices de transformación)
    let mut transformed_vertices = Vec::with_capacity(vertex_array.len());
    for vertex in vertex_array {
        let transformed = vertex_shader(vertex, uniforms);
        transformed_vertices.push(transformed);
    }

    // Ensamblaje de primitivas: agrupar vértices en triángulos
    let mut triangles = Vec::new();
    for i in (0..transformed_vertices.len()).step_by(3) {
        if i + 2 < transformed_vertices.len() {
            triangles.push([
                transformed_vertices[i].clone(),
                transformed_vertices[i + 1].clone(),
                transformed_vertices[i + 2].clone(),
            ]);
        }
    }

    // Rasterización: convertir triángulos a fragmentos (píxeles)
    let mut fragments = Vec::new();
    for tri in &triangles {
        fragments.extend(triangle(&tri[0], &tri[1], &tri[2]));
    }

    // Procesamiento de fragmentos: sombrear cada fragmento y dibujarlo en el framebuffer
    for fragment in fragments {
        let x = fragment.position.x as usize;
        let y = fragment.position.y as usize;

        if x < framebuffer.width && y < framebuffer.height {
            // Aplicamos el fragment shader
            let shaded_color = fragment_shader(&fragment, &uniforms);
            let color = shaded_color.to_hex();

            // Dibujamos el píxel con el color sombreado en el framebuffer
            framebuffer.set_current_color(color);
            framebuffer.point(x, y, fragment.depth);
        }
    }
}


fn load_panoramic_image(path: &str) -> DynamicImage {
    open(path).unwrap()
}

fn project_to_image(x_angle: f32, y_angle: f32, image_width: u32, image_height: u32) -> (u32, u32) {
    // Convierte los ángulos a un rango de 0 a 1
    let x_normalized = (x_angle + 180.0) / 360.0; // -180 a 180 -> 0 a 1
    let y_normalized = (y_angle + 90.0) / 180.0; // -90 a 90 -> 0 a 1
    
    // Calcula las coordenadas de la imagen
    let x_pixel = (x_normalized * image_width as f32) as u32;
    let y_pixel = (y_normalized * image_height as f32) as u32;

    (x_pixel, y_pixel)
}
//Planetas
// Definición de un planeta
pub struct Planet {
    position: Vec3,
    rotation_speed: f32,
    orbit_radius: f32,    // Radio de la órbita
    orbit_speed: f32,
    orbit_phase: f32,    
    scale: f32,
    shader_mode: u8,
}

// Generar planetas
pub fn generate_planets() -> Vec<Planet> {
    let mut rng = rand::thread_rng();

    vec![
        Planet { 
            position: Vec3::new(0.0, 0.0, 0.0), // Sol
            rotation_speed: 0.0, 
            orbit_radius: 0.0,  // El Sol no orbita
            orbit_speed: 0.0, 
            orbit_phase: 0.0,  // No aplica al Sol
            scale: 4.0, 
            shader_mode: 1, 
        },
        Planet { 
            position: Vec3::new(10.0, 0.0, 0.0), 
            rotation_speed: 0.1, 
            orbit_radius: 10.0,
            orbit_speed: 0.02,
            orbit_phase: rng.gen_range(0.0..(2.0 * std::f32::consts::PI)),  // Ángulo inicial aleatorio
            scale: 2.4, 
            shader_mode: 2, 
        },
        Planet { 
            position: Vec3::new(0.0, 0.0, 18.0),
            rotation_speed: 0.1, 
            orbit_radius: 15.0,
            orbit_speed: 0.01,
            orbit_phase: rng.gen_range(0.0..(2.0 * std::f32::consts::PI)),  // Ángulo inicial aleatorio
            scale: 1.8, 
            shader_mode: 4, 
        },
        Planet { 
            position: Vec3::new(-26.0, 0.0, 0.0), 
            rotation_speed: 0.1, 
            orbit_radius: 23.8,
            orbit_speed: 0.015,
            orbit_phase: rng.gen_range(0.0..(2.0 * std::f32::consts::PI)),  // Ángulo inicial aleatorio
            scale: 2.2, 
            shader_mode: 6, 
        },
        Planet { 
            position: Vec3::new(0.0, 0.0, -34.0), 
            rotation_speed: 0.01, 
            orbit_radius: 29.2,
            orbit_speed: 0.015,
            orbit_phase: rng.gen_range(0.0..(2.0 * std::f32::consts::PI)),  // Ángulo inicial aleatorio
            scale: 1.5, 
            shader_mode: 5, 
        },
    ]
}

fn calculate_planet_transformations(planets: &[Planet], time: u32) -> Vec<(Vec3, Vec3, f32)> {
    planets.iter().map(|planet| {
        let angle = planet.orbit_speed * time as f32 + planet.orbit_phase; // Considera el desfase inicial
        let orbit_x = planet.orbit_radius * angle.cos();  // Posición X en la órbita circular
        let orbit_z = planet.orbit_radius * angle.sin();  // Posición Z en la órbita circular

        // Devolvemos la nueva posición y transformaciones
        (
            Vec3::new(orbit_x, 0.0, orbit_z),
            Vec3::new(0.0, planet.rotation_speed * time as f32, 0.0),
            planet.scale,
        )
    }).collect()
}

fn update_planets(planets: &mut Vec<Planet>, delta_time: f32) {
    for planet in planets.iter_mut() {
        // Calculamos el ángulo de órbita en función del tiempo
        planet.position.x = planet.orbit_radius * planet.orbit_speed * delta_time.cos();
        planet.position.z = planet.orbit_radius * planet.orbit_speed * delta_time.sin();
        
        // Actualizamos la rotación del planeta alrededor de su eje
        planet.position.x += planet.rotation_speed * delta_time;
    }
}

struct WarpDestination {
    position: Vec3,
    target: Vec3,  // El punto al que apunta la cámara
}

fn define_warp_positions(planets: &[Planet]) -> Vec<WarpDestination> {
    vec![
        // Vista general de todos los planetas (por encima del sistema solar)
        WarpDestination {
            position: Vec3::new(0.0, 50.0, 50.0),  // Vista aérea
            target: Vec3::new(0.0, 0.0, 0.0),      // Apunta al centro del sistema solar
        },
        // Warp al Sol
        WarpDestination {
            position: Vec3::new(0.0, 0.0, 6.0),  // Cerca del Sol
            target: Vec3::new(0.0, 0.0, 0.0),    // Apunta al centro del Sol
        },
        // Warp al planeta rocoso
        WarpDestination {
            position: Vec3::new(24.834557, 3.2328124, 0.9333064),  // A una distancia razonable del planeta rocoso
            target: Vec3::new(10.0, 0.0, 0.0),    // Apunta al planeta rocoso
        },
        // Warp al planeta helado
        WarpDestination {
            position: Vec3::new(-1.149943e-6, 2.3341978, 44.307625), // Más alejado del planeta helado
            target: Vec3::new(0.0, 0.0, 18.0),    // Apunta al planeta helado
        },
    ]
}


fn main() {
    let mut current_warp_index = 0; // Nuevo índice para el destino warp

    let mut planets = generate_planets();
    let warp_destinations = define_warp_positions(&planets);


    let image = load_panoramic_image("assets/image/space.png");
    let (image_width, image_height) = image.dimensions();

    let window_width = 800;
    let window_height = 600;
    let framebuffer_width = 800;
    let framebuffer_height = 600;
    let frame_delay = Duration::from_millis(16);

    // Crear un framebuffer para el renderizado
    let mut framebuffer = Framebuffer::new(framebuffer_width, framebuffer_height);

    // Crear una ventana para mostrar la salida
    let mut window = Window::new(
        "Space Travel",
        window_width,
        window_height,
        WindowOptions::default(),
    )
    .unwrap();

    window.set_position(500, 500);
    window.update();

    framebuffer.set_background_color(0x000000);

    // Parámetros de la cámara
    let mut camera = Camera::new(
        Vec3::new(0.0, 0.0, 5.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    );

    let mut current_position = camera.eye;
    let mut current_target = camera.center;

    // Cargar la esfera desde el archivo OBJ
    let sphere_obj = Obj::load("assets/models/sphere-1.obj").expect("Error al cargar sphere-1.obj");
    let vertex_array_sphere = sphere_obj.get_vertex_array();

    // Cargar la nave desde el archivo OBJ
    let ship_obj = Obj::load("assets/models/nave.obj").expect("Error al cargar nave.obj");
    let vertex_array_ship = ship_obj.get_vertex_array();

    let mut time = 0;
    let shader_mode = 0;

    while window.is_open() {
        let delta_time = 1.0 / 60.0; // Tiempo entre frames (aproximado)

        // Manejar la entrada del usuario
        handle_input(
            &window,
            &mut camera,
            &warp_destinations,
            delta_time,
            &mut current_position,
            &mut current_target,
            &mut current_warp_index,
        );

        time += 1;

        framebuffer.clear();

        update_planets(&mut planets, delta_time);

        // Crear las matrices de transformación para la esfera
        let translation_sphere = Vec3::new(0.0, 0.0, 0.0);
        let rotation_sphere = Vec3::new(0.0, 0.0, 0.0);
        let scale_sphere = 1.0f32;

        let model_matrix_sphere =
            create_model_matrix(translation_sphere, scale_sphere, rotation_sphere);

        // Crear las matrices de transformación para la nave

        // Calcular la posición de la nave en relación con la cámara
        let camera_forward = (camera.center - camera.eye).normalize(); // Dirección en la que mira la cámara
        let offset = camera_forward * 1.5; // Posición de la nave, 2 unidades delante de la cámara
        let translation_ship = camera.eye + camera_forward * 1.5 + Vec3::new(0.0, -0.5, 0.0);

        let rotation_ship = Vec3::new(0.0, 0.0, 0.0); // Rotación animada
        let scale_ship = 0.05f32;

        let model_matrix_ship = create_model_matrix(translation_ship, scale_ship, rotation_ship);

        let view_matrix = create_view_matrix(camera.eye, camera.center, camera.up);
        let projection_matrix =
            create_perspective_matrix(window_width as f32, window_height as f32);
        let viewport_matrix =
            create_viewport_matrix(framebuffer_width as f32, framebuffer_height as f32);

        // Preparar las uniformes para el shader
        let uniforms_sphere = Uniforms {
            model_matrix: model_matrix_sphere,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            time,
            noise: create_noise(),
            shader_mode,
        };

        // Preparar las uniformes para la nave
        let uniforms_ship = Uniforms {
            model_matrix: model_matrix_ship,
            view_matrix,
            projection_matrix,
            viewport_matrix,
            time,
            noise: create_noise(),
            shader_mode: 8,  
        };

        // Renderizar la esfera
        render(
            &mut framebuffer,
            &uniforms_sphere,
            &vertex_array_sphere,
            &image,
            image_width,
            image_height,
        );

        // Renderizar la nave
        render(
            &mut framebuffer,
            &uniforms_ship,
            &vertex_array_ship,
            &image,
            image_width,
            image_height,
        );

        // Obtener las transformaciones para los planetas
        let transformations = calculate_planet_transformations(&planets, time);
        for (planet, (translation, rotation, scale)) in planets.iter().zip(transformations) {
            let model_matrix = create_model_matrix(translation, scale, rotation);

            let uniforms = Uniforms {
                model_matrix,
                view_matrix,
                projection_matrix,
                viewport_matrix,
                time,
                noise: create_noise(),
                shader_mode: planet.shader_mode,
            };


            render(
                &mut framebuffer,
                &uniforms,
                &vertex_array_sphere, // Usa la esfera como modelo base para los planetas
                &image,
                image_width,
                image_height,
            );
        }

        // Actualizar la ventana con el contenido del framebuffer
        window
            .update_with_buffer(&framebuffer.buffer, framebuffer_width, framebuffer_height)
            .unwrap();

        std::thread::sleep(frame_delay);
    }
}

fn handle_input(
    window: &Window,
    camera: &mut Camera,
    warp_destinations: &[WarpDestination],
    delta_time: f32,
    current_position: &mut Vec3,
    current_target: &mut Vec3,
    current_warp_index: &mut usize, 
) {
    let movement_speed = 1.0;
    let rotation_speed = std::f32::consts::PI / 50.0;
    let zoom_speed = 0.1;
    let keys = [Key::Key1, Key::Key2, Key::Key3, Key::Key4];

    // Camera orbit controls
    if window.is_key_down(Key::Left) {
        camera.orbit(rotation_speed, 0.0);
    }
    if window.is_key_down(Key::Right) {
        camera.orbit(-rotation_speed, 0.0);
    }
    if window.is_key_down(Key::W) {
        camera.orbit(0.0, -rotation_speed);
    }
    if window.is_key_down(Key::S) {
        camera.orbit(0.0, rotation_speed);
    }

    // Camera movement controls
    let mut movement = Vec3::new(0.0, 0.0, 0.0);
    if window.is_key_down(Key::A) {
        movement.x -= movement_speed;
    }
    if window.is_key_down(Key::D) {
        movement.x += movement_speed;
    }
    if window.is_key_down(Key::Q) {
        movement.y += movement_speed;
    }
    if window.is_key_down(Key::E) {
        movement.y -= movement_speed;
    }
    if movement.magnitude() > 0.0 {
        camera.move_center(movement);
    }

    // Camera zoom controls
    if window.is_key_down(Key::Up) {
        camera.zoom(zoom_speed);
    }
    if window.is_key_down(Key::Down) {
        camera.zoom(-zoom_speed);
    }

    // Detectar teclas para activar el warp
    for (i, key) in keys.iter().enumerate() {
        if window.is_key_pressed(*key, KeyRepeat::No) {
            *current_warp_index = i.min(warp_destinations.len() - 1); // Prevenir desbordamientos

            // Salto instantáneo al destino
            let target_position = warp_destinations[*current_warp_index].position;
            let target_target = warp_destinations[*current_warp_index].target;

            camera.eye = target_position;
            camera.center = target_target;

            *current_position = camera.eye;    // Actualizar la posición actual
            *current_target = camera.center;  // Actualizar el objetivo actual
        }
    }
}
