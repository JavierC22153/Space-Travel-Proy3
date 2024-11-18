
use nalgebra_glm::{Vec3, Vec4, Mat3, dot, mat4_to_mat3};
use crate::vertex::Vertex;
use crate::Uniforms;
use crate::fragment::Fragment;
use crate::color::Color;

pub fn vertex_shader(vertex: &Vertex, uniforms: &Uniforms) -> Vertex {
    let position = Vec4::new(
        vertex.position.x,
        vertex.position.y,
        vertex.position.z,
        1.0
    );

    let transformed = uniforms.projection_matrix * uniforms.view_matrix * uniforms.model_matrix * position;

    let w = transformed.w;
    let transformed_position = Vec4::new(
        transformed.x / w,
        transformed.y / w,
        transformed.z / w,
        1.0
    );

    let screen_position = uniforms.viewport_matrix * transformed_position;

    let model_mat3 = mat4_to_mat3(&uniforms.model_matrix);
    let normal_matrix = model_mat3.transpose().try_inverse().unwrap_or(Mat3::identity());

    let transformed_normal = normal_matrix * vertex.normal;

    Vertex {
        position: vertex.position,
        normal: vertex.normal,
        tex_coords: vertex.tex_coords,
        color: vertex.color,
        transformed_position: Vec3::new(screen_position.x, screen_position.y, screen_position.z),
        transformed_normal: transformed_normal
    }
}

pub fn fragment_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  match uniforms.shader_mode {
      1 => star_shader(fragment, uniforms),        // Sol
      2 => broken_terrain_shader(fragment, uniforms), // Planeta rocoso
      3 => gas_giant_shader(fragment, uniforms),  // Gigante gaseoso
      4 => icy_planet_shader(fragment, uniforms), // Planeta helado
      5 => volcanic_planet_shader(fragment, uniforms), // Planeta volcánico
      6 => earth_like_planet_shader(fragment, uniforms), // Planeta Tierra
      7 => alien_planet_shader(fragment, uniforms), // Planeta Alienigena
      8 => spaceship_shader(fragment, uniforms), // Nave
      _ => Color::new(0, 0, 0) // Shader por defecto (negro)
  }
}

fn star_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let bright_color = Color::new(255, 223, 0); // Bright yellow
  let dark_color = Color::new(255, 69, 0); // Fiery orange-red

  // Pulsating noise-based effect
  let zoom = 300.0;
  let t = uniforms.time as f32 * 0.05;
  let noise_value = uniforms.noise.get_noise_3d(
      fragment.vertex_position.x * zoom,
      fragment.vertex_position.y * zoom,
      t,
  );

  let intensity = (t * 2.0).sin() * 0.3 + 0.7;
  let color = dark_color.lerp(&bright_color, noise_value) * intensity;

  color * fragment.intensity
}

pub fn gas_giant_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  // Colores para las capas del planeta
  let color_layer_1 = Color::new(255, 165, 0); // Naranja
  let color_layer_2 = Color::new(255, 215, 0); // Amarillo dorado
  let color_layer_3 = Color::new(255,200, 26); // Amarillo claro
  let color_layer_4 = Color::new(255, 179, 34); // Naranja pálido

  // Colores para los anillos
  let ring_color = Color::new(238, 238, 214); // Color gris claro para los anillos

  // Parámetros de zoom para diferentes niveles de detalle
  let zoom_planet = 10.0; // Zoom para las capas del planeta
  let zoom_ring = 5.0;    // Zoom para los anillos
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;
  let t = uniforms.time as f32 * 0.5; // Movimiento en el tiempo
  let ox = 25.0;    
  let oy = 25.0;    


  // Ruido para las capas del planeta
  let noise_value = uniforms.noise.get_noise_2d(x * zoom_planet * ox, y * zoom_planet * oy).abs();
  let layer_color = if noise_value < 0.15 {
      color_layer_1
  } else if noise_value < 0.7 {
      color_layer_2
  } else if noise_value < 0.75 {
      color_layer_3
  } else {
      color_layer_4
  };

  // Añadir variación de ruido para el efecto de nubes
  let cloud_noise = uniforms.noise.get_noise_2d(x * zoom_planet + 100.0, y * zoom_planet + 100.0).abs() * 0.3;
  let cloud_color = layer_color.lerp(&Color::new(255, 255, 255), cloud_noise * 0.1); // Menos mezcla con blanco
  let planet_color = cloud_color * (fragment.intensity * 3.0).clamp(0.0, 1.0);

  // Añadir anillos con movimiento
  let ring_distance = 0.05; // Posición del anillo
  let ring_noise_value = uniforms.noise.get_noise_2d(
      x * zoom_ring + 50.0 + t,
      y * zoom_ring + 50.0 + t,
  ).abs() * 0.5;

  // Crear la apariencia de anillos
  let ring_intensity = if (fragment.vertex_position.y - ring_distance).abs() < 0.05 {
      ring_color.lerp(&Color::black(), ring_noise_value * 0.7)
  } else {
      Color::black() // No hay anillo aquí
  };

  // Mezcla el color del planeta con los anillos
  let final_color = planet_color.blend(ring_intensity, 0.4);

  // Devuelve el color final del fragmento
  final_color
}

pub fn icy_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  // Generar ruido para la base y los detalles
  let base_noise = uniforms.noise.get_noise_2d(fragment.vertex_position.x * 5.0, fragment.vertex_position.y * 5.0);
  let detail_noise = uniforms.noise.get_noise_2d(fragment.vertex_position.x * 10.0, fragment.vertex_position.y * 10.0);
  
  // Colores base y de resaltado
  let ice_color = Color::new(173, 216, 230); // Azul claro
  let highlight_color = Color::new(255, 255, 255); // Blanco
  let shadow_color = Color::new(100, 149, 237); // Azul más oscuro para sombras

  // Combina los colores de hielo y resaltado según el valor de ruido
  let base_color = ice_color.lerp(&highlight_color, base_noise);
  
  // Agregar variación en el color usando el ruido de detalles
  let detail_variation = detail_noise * 0.2; // Ajuste de la intensidad del detalle
  let color_with_detail = base_color.lerp(&shadow_color, detail_variation);
  
  // Añadir iluminación especular
  let light_dir = Vec3::new(1.0, 1.0, 0.5).normalize();
  let normal = fragment.normal.normalize();
  let diffuse_intensity = dot(&normal, &light_dir).max(0.0);
  
  // Brillo especular para simular reflejos en el hielo
  let specular_intensity = (dot(&normal, &(light_dir * -1.0))).max(0.0).powi(16); // Aumentar el brillo
  let specular_color = highlight_color * specular_intensity;

  // Combina la luz difusa y la especular con el color base
  let lit_color = color_with_detail * (0.4 + 0.6 * diffuse_intensity) + specular_color;

  // Translucidez para simular hielo
  let translucency = (base_noise * 0.5 + 0.5).clamp(0.0, 1.0); // Ajustar la translucidez
  let final_color = lit_color.lerp(&Color::new(255, 255, 255), translucency * 0.2); // Añadir un ligero brillo

  final_color * fragment.intensity
}

fn volcanic_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  // Colores base
  let bright_color = Color::new(255, 240, 0); // Color brillante (lava)
  let dark_color = Color::new(130, 20, 0);    // Color oscuro (rojo-naranja)
  let base_color = Color::new(30, 30, 30);     // Color base oscuro para el planeta
  let lava_color = Color::new(255, 100, 0);    // Color de lava brillante

  // Obtener posición del fragmento
  let position = Vec3::new(
      fragment.vertex_position.x,
      fragment.vertex_position.y,
      fragment.depth,
  );

  // Frecuencia base y amplitud para el efecto de pulsación
  let base_frequency = 0.2;
  let pulsate_amplitude = 0.5;
  let t = uniforms.time as f32 * 0.01;

  // Pulsar en el eje z para cambiar el tamaño de las manchas
  let pulsate = (t * base_frequency).sin() * pulsate_amplitude;

  // Aplicar ruido a las coordenadas con sutil pulsación en el eje z
  let zoom = 1000.0; // Factor de zoom constante
  let noise_value1 = uniforms.noise.get_noise_3d(
      position.x * zoom,
      position.y * zoom,
      (position.z + pulsate) * zoom,
  );
  let noise_value2 = uniforms.noise.get_noise_3d(
      (position.x + 1000.0) * zoom,
      (position.y + 1000.0) * zoom,
      (position.z + 1000.0 + pulsate) * zoom,
  );
  let lava_noise_value = (noise_value1 + noise_value2) * 0.5; // Promediar el ruido para transiciones más suaves

  // Generar capas de ruido para detalles adicionales
  let detail_noise = uniforms.noise.get_noise_2d(fragment.vertex_position.x * 20.0, fragment.vertex_position.y * 20.0);
  let rough_noise = uniforms.noise.get_noise_2d(fragment.vertex_position.x * 5.0, fragment.vertex_position.y * 5.0);

  // Detalles de superficie volcánica utilizando múltiples capas de ruido
  let surface_detail = base_color.lerp(&dark_color, rough_noise * 0.5);
  let lava_detail = lava_color.lerp(&Color::new(255, 140, 0), detail_noise * 0.5);

  // Color final que combina detalles de lava y roca
  let final_color = surface_detail.lerp(&lava_detail, lava_noise_value * fragment.intensity);

  // Aumentar la intensidad de la lava en ciertas áreas
  let lava_threshold = 0.5; // Umbral para determinar si hay lava visible
  let output_color = if lava_noise_value > lava_threshold {
      final_color.lerp(&bright_color, 0.5) // Añadir un brillo de lava donde hay actividad volcánica
  } else {
      final_color
  };

  output_color
}


fn earth_like_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let zoom = 170.0;
  let ox = 700.0;
  let oy = 600.0;
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;
  let t = uniforms.time as f32 * 0.5;  // Para el movimiento de las nubes en el tiempo

  // Ruido base para el terreno
  let noise_value = uniforms.noise.get_noise_2d(x * zoom + ox, y * zoom + oy);

  // Umbrales para agua y tierra
  let water_threshold = 0.50;
  let land_threshold = 0.55;

  // Colores para el terreno
  let water_color = Color::new(13, 105, 171); // Azul profundo del océano
  let shore_color = Color::new(244, 164, 96); // Arena para la orilla
  let land_color = Color::new(34, 139, 34);   // Verde para la tierra

  // Selección del color base según el ruido
  let base_color = if noise_value < water_threshold {
      water_color
  } else if noise_value < land_threshold {
      shore_color
  } else {
      land_color
  };

  // Iluminación del terreno
  let light_dir = Vec3::new(1.0, 1.0, 0.5).normalize();
  let normal = fragment.normal.normalize();
  let diffuse_intensity = dot(&normal, &light_dir).max(0.0);
  let lit_color = base_color * (0.4 + 0.6 * diffuse_intensity); // Luz ambiente + luz difusa

  // Capa de nubes con mayor densidad y forma distinta
  let cloud_zoom = 20.0;  // Mayor zoom para mayor cantidad de nubes
  let cloud_offset = 37.0; // Offset para diferenciar su forma de los continentes

  // Transformación para dar forma y aumentar la densidad de las nubes
  let oval_x = (x * cloud_zoom * 0.8 + ox + t).sin() * cloud_offset; // Sinusoide para distorsión
  let oval_y = (y * cloud_zoom * 0.95 + oy).cos() * cloud_offset;      // Sinusoide para distorsión

  // Obtener el valor de ruido modificado para las nubes
  let cloud_noise_value = uniforms.noise.get_noise_2d(oval_x, oval_y);

  // Umbral ajustado para nubes más densas
  let cloud_threshold = 0.45; // Umbral más bajo para mayor densidad de nubes
  let cloud_color = Color::new(255, 255, 255); // Blanco para las nubes

  // Superponer nubes si el ruido excede el umbral, con transparencia
  let final_color = if cloud_noise_value > cloud_threshold {
      lit_color.blend(cloud_color, 0.5) // Mezcla con el color base con 50% de opacidad
  } else {
      lit_color
  };

  final_color * fragment.intensity
}


pub fn broken_terrain_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let base_color = Color::new(140, 130, 120); // Color base del terreno
  let crack_color = Color::new(200, 200, 255); // Color de las grietas
  let dirt_color = Color::new(100, 70, 40); // Color de la tierra

  // Parámetros de zoom
  let zoom_2d = 10.0;
  let crack_zoom = 30.0;

  // Generar ruido para el terreno
  let ox = 0.0; // Desplazamiento en x, puedes ajustarlo según sea necesario
  let oy = 0.0; // Desplazamiento en y, puedes ajustarlo según sea necesario
  let noise_value = uniforms.noise.get_noise_2d(
      fragment.vertex_position.x * zoom_2d + ox,
      fragment.vertex_position.y * zoom_2d + oy
  );

  // Generar ruido para las grietas
  let crack_noise_value = uniforms.noise.get_noise_2d(
      fragment.vertex_position.x * crack_zoom + ox,
      fragment.vertex_position.y * crack_zoom + oy
  );

  // Determinar el color en base al ruido
  let terrain_color = if noise_value < 0.2 {
      dirt_color // Terreno quebrado
  } else {
      base_color // Terreno normal
  };

  // Calcular la intensidad de las grietas
  let crack_intensity = crack_noise_value.abs().clamp(0.0, 1.0);
  let final_color = terrain_color.lerp(&crack_color, crack_intensity);

  // Aplicar intensidad del fragmento
  final_color * fragment.intensity
}

fn alien_planet_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
  let zoom = 50.0;
  let x = fragment.vertex_position.x;
  let y = fragment.vertex_position.y;

  // Ruido para la superficie del planeta
  let noise_value = uniforms.noise.get_noise_2d(x * zoom, y * zoom);

  // Colores vibrantes para las diferentes capas del planeta
  let base_color = Color::new(75, 0, 130); // Púrpura oscuro
  let secondary_color = Color::new(0, 255, 255); // Cian brillante
  let tertiary_color = Color::new(243, 22, 206); 

  // Selección del color base según el ruido, creando capas en el planeta
  let planet_color = if noise_value < 0.3 {
      base_color
  } else if noise_value < 0.6 {
      secondary_color
  } else {
      tertiary_color
  };

  // Efecto de emisión ajustado
  let emission_strength = (0.5 + noise_value * 1.5).clamp(0.0, 1.0); // Intensidad de la emisión
  let emission_color = Color::new(255, 20, 147) * emission_strength; // Color de emisión (rosa brillante)

  // Mezcla del color del planeta con el color de emisión
  let final_color = planet_color * 0.7 + emission_color * 0.3;

  // Configuración de la iluminación
  let light_dir = Vec3::new(1.0, 1.0, 0.5).normalize();
  let normal = fragment.normal.normalize();
  let diffuse_intensity = dot(&normal, &light_dir).max(0.0);

  // Aplicación de iluminación ambiente y difusa
  let ambient_intensity = 0.4; // Intensidad de la luz ambiente
  let diffuse_factor = 0.6;    // Factor de luz difusa
  
  let lit_color = final_color * (ambient_intensity + diffuse_factor * diffuse_intensity);

  // Devolver el color final del fragmento
  lit_color * fragment.intensity
}


fn spaceship_shader(fragment: &Fragment, uniforms: &Uniforms) -> Color {
    // Colores base del material
    let base_color = Color::new(200, 200, 200); // Gris claro
    let highlight_color = Color::new(255, 255, 255); // Blanco para los reflejos
    let shadow_color = Color::new(120, 120, 120); // Gris oscuro para las sombras

    // Generar ruido para agregar variaciones sutiles al material
    let zoom = 15.0; // Zoom para el ruido
    let noise_value = uniforms.noise.get_noise_2d(
        fragment.vertex_position.x * zoom,
        fragment.vertex_position.y * zoom,
    );

    // Agregar variación al color base
    let color_variation = base_color.lerp(&shadow_color, noise_value * 0.1);

    // Iluminación del material
    let light_dir = Vec3::new(1.0, 1.0, 0.5).normalize(); // Dirección de la luz
    let normal = fragment.normal.normalize(); // Normal del fragmento
    let diffuse_intensity = dot(&normal, &light_dir).max(0.0); // Intensidad difusa

    // Brillo especular para simular un material metálico suave
    let view_dir = Vec3::new(0.0, 0.0, 1.0); // Dirección de la cámara
    let reflect_dir = 2.0 * dot(&normal, &light_dir) * normal - light_dir; // Reflexión de la luz
    let specular_intensity = dot(&reflect_dir, &view_dir).max(0.0).powi(16); // Brillo especular
    let specular_color = highlight_color * specular_intensity;

    // Combinar iluminación difusa y especular con el color base
    let lit_color = color_variation * (0.4 + 0.6 * diffuse_intensity) + specular_color;

    // Ajustar la intensidad del color según la iluminación del fragmento
    let final_color = lit_color * fragment.intensity;

    final_color
}
