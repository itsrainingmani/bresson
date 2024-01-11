use std::f32::consts::PI;

const DW: usize = 4;
const DH: usize = 8;

static EARTH_DAY: &str = include_str!("../texture/earth.txt");
static EARTH_NIGHT: &str = include_str!("../texture/earth_night.txt");

pub struct Canvas {
    pub matrix: Vec<Vec<char>>,
    pub size: (usize, usize),
    pub char_size: (usize, usize),
}

impl Canvas {
    pub fn new(x: usize, y: usize, cs: Option<(usize, usize)>) -> Self {
        let matrix = vec![vec![' '; x]; y];
        Self {
            matrix,
            size: (x, y),
            char_size: cs.unwrap_or((DW, DH)),
        }
    }

    pub fn get_size(&self) -> (usize, usize) {
        self.size
    }

    pub fn clear(&mut self) {
        for i in self.matrix.iter_mut().flatten() {
            *i = ' ';
        }
    }

    pub fn draw_at(&mut self, row: usize, col: usize, c: char) {
        if row >= self.size.0 || col >= self.size.1 {
            return;
        }

        self.matrix[col][row] = c;
    }
}

#[derive(Default)]
pub struct Camera {
    x: f32,
    y: f32,
    z: f32,
    matrix: [f32; 16],
    inv: [f32; 16],
}

impl Camera {
    pub fn update(&mut self, r: f32, alfa: f32, beta: f32) {
        let (a, b, c, d) = (alfa.sin(), alfa.cos(), beta.sin(), beta.cos());
        let x = r * b * d;
        let y = r * a * d;
        let z = r * c;

        let mut matrix = [0.0; 16];
        matrix[3] = 0.0;
        matrix[7] = 0.0;
        matrix[11] = 0.0;
        matrix[15] = 1.0;
        matrix[0] = -a;
        matrix[1] = b;
        matrix[2] = 0.0;
        matrix[4] = b * c;
        matrix[5] = a * c;
        matrix[6] = -d;
        matrix[8] = b * d;
        matrix[9] = a * d;
        matrix[10] = c;
        matrix[12] = x;
        matrix[13] = y;
        matrix[14] = z;

        let mut inv = [0.; 16];
        invert(&mut inv, matrix);

        self.x = x;
        self.y = y;
        self.z = z;
        self.matrix = matrix;
        self.inv = inv;
    }
}

pub enum TextureType {
    Day,
    Night,
}

pub struct Globe {
    pub camera: Camera,
    pub radius: f32,
    pub angle: f32,
    pub display_night: bool,
    palette: Vec<char>,
    day_texture: Vec<Vec<char>>,
    night_texture: Vec<Vec<char>>,
}

impl Globe {
    pub fn new(radius: f32, angle: f32, display_night: bool) -> Self {
        let day_texture = Globe::load_texture(TextureType::Day);
        let night_texture = Globe::load_texture(TextureType::Night);
        let palette = vec![
            ' ', '.', ':', ';', '\'', ',', 'w', 'i', 'o', 'g', 'O', 'L', 'X', 'H', 'W', 'Y', 'V',
            '@',
        ];

        Self {
            camera: Camera::default(),
            radius,
            angle,
            display_night,
            palette,
            day_texture,
            night_texture,
        }
    }

    pub fn toggle_night(&mut self) {
        self.display_night = !self.display_night;
    }

    fn load_texture(tex: TextureType) -> Vec<Vec<char>> {
        let texture_data = match tex {
            TextureType::Day => EARTH_DAY,
            TextureType::Night => EARTH_NIGHT,
        };

        let mut data = Vec::new();
        let lines = texture_data.lines();

        for line in lines {
            let row: Vec<char> = line.chars().rev().collect();
            data.push(row);
        }

        data
    }

    pub fn texture_size(&self) -> (usize, usize) {
        (self.day_texture[0].len(), self.day_texture.len())
    }

    pub fn render_sphere(&self, canvas: &mut Canvas) {
        let light = [0.0, 999999.0, 0.0];
        let (width, height) = canvas.get_size();
        let (c_w, c_h) = canvas.char_size;
        for yi in 0..height {
            let yif = yi as isize;
            for xi in 0..width {
                let xif = xi as isize;
                // Origin of the Ray
                let o = [self.camera.x, self.camera.y, self.camera.z];

                // Unit vector. direction of the Ray
                let mut u = [
                    -((xif - (width / c_w / 2) as isize) as f32 + 0.5) / (width / c_w / 2) as f32,
                    ((yif - (height / c_h / 2) as isize) as f32 + 0.5) / (height / c_h / 2) as f32,
                    -1.0,
                ];
                transform_vector(&mut u, self.camera.matrix);
                u[0] -= self.camera.x;
                u[1] -= self.camera.y;
                u[2] -= self.camera.z;
                normalize(&mut u);
                let discriminant = dot(&u, &o).powi(2) - dot(&o, &o) + self.radius.powi(2);

                // Ray doesn't hit the sphere
                if discriminant < 0.0 {
                    continue;
                }

                let distance = -discriminant.sqrt() - dot(&u, &o);

                // Intersection Point
                let inter = [
                    o[0] + distance * u[0],
                    o[1] + distance * u[1],
                    o[2] + distance * u[2],
                ];

                // Surface normal
                let mut n = [inter[0], inter[1], inter[2]];
                normalize(&mut n);

                // Unit vector pointing from intersection to light source
                let mut l = [
                    light[0] - inter[0],
                    light[1] - inter[1],
                    light[2] - inter[2],
                ];
                normalize(&mut l);

                let luminance = clamp(5.0 * dot(&n, &l) + 0.5, 0.0, 1.0);
                let mut temp = [inter[0], inter[1], inter[2]];
                rotate_x(&mut temp, -PI * 2.0 * 0. / 360.0);

                // Computing coordinates for sphere
                let phi = -temp[2] / self.radius / 2.0 + 0.5;
                let mut theta = (temp[1] / temp[0]).atan() / PI + 0.5 + self.angle / 2.0 / PI;
                theta -= theta.floor();

                let (tex_x, tex_y) = self.texture_size();
                let earth_x = (theta * tex_x as f32) as usize;
                let earth_y = (phi * tex_y as f32) as usize;

                if self.display_night {
                    let day = find_index(self.day_texture[earth_y][earth_x], &self.palette);

                    let night = find_index(self.night_texture[earth_y][earth_x], &self.palette);
                    let mut index =
                        ((1.0 - luminance) * night as f32 + luminance * day as f32) as usize;
                    if index >= self.palette.len() {
                        index = 0;
                    }
                    canvas.draw_at(xi, yi, self.palette[index]);
                } else {
                    canvas.draw_at(xi, yi, self.day_texture[earth_y][earth_x]);
                }
            }
        }
    }
}

fn find_index(target: char, palette: &[char]) -> isize {
    for (i, &ch) in palette.iter().enumerate() {
        if target == ch {
            return i as isize;
        }
    }

    -1
}

fn transform_vector(vec: &mut [f32; 3], m: [f32; 16]) {
    let (tx, ty, tz) = (
        vec[0] * m[0] + vec[1] * m[4] + vec[2] * m[8] + m[12],
        vec[0] * m[1] + vec[1] * m[5] + vec[2] * m[9] + m[13],
        vec[0] * m[2] + vec[1] * m[6] + vec[2] * m[10] + m[14],
    );
    vec[0] = tx;
    vec[1] = ty;
    vec[2] = tz;
}

fn invert(inv: &mut [f32; 16], matrix: [f32; 16]) {
    inv[0] = matrix[5] * matrix[10] * matrix[15]
        - matrix[5] * matrix[11] * matrix[14]
        - matrix[9] * matrix[6] * matrix[15]
        + matrix[9] * matrix[7] * matrix[14]
        + matrix[13] * matrix[6] * matrix[11]
        - matrix[13] * matrix[7] * matrix[10];

    inv[4] = -matrix[4] * matrix[10] * matrix[15]
        + matrix[4] * matrix[11] * matrix[14]
        + matrix[8] * matrix[6] * matrix[15]
        - matrix[8] * matrix[7] * matrix[14]
        - matrix[12] * matrix[6] * matrix[11]
        + matrix[12] * matrix[7] * matrix[10];

    inv[8] = matrix[4] * matrix[9] * matrix[15]
        - matrix[4] * matrix[11] * matrix[13]
        - matrix[8] * matrix[5] * matrix[15]
        + matrix[8] * matrix[7] * matrix[13]
        + matrix[12] * matrix[5] * matrix[11]
        - matrix[12] * matrix[7] * matrix[9];

    inv[12] = -matrix[4] * matrix[9] * matrix[14]
        + matrix[4] * matrix[10] * matrix[13]
        + matrix[8] * matrix[5] * matrix[14]
        - matrix[8] * matrix[6] * matrix[13]
        - matrix[12] * matrix[5] * matrix[10]
        + matrix[12] * matrix[6] * matrix[9];

    inv[1] = -matrix[1] * matrix[10] * matrix[15]
        + matrix[1] * matrix[11] * matrix[14]
        + matrix[9] * matrix[2] * matrix[15]
        - matrix[9] * matrix[3] * matrix[14]
        - matrix[13] * matrix[2] * matrix[11]
        + matrix[13] * matrix[3] * matrix[10];

    inv[5] = matrix[0] * matrix[10] * matrix[15]
        - matrix[0] * matrix[11] * matrix[14]
        - matrix[8] * matrix[2] * matrix[15]
        + matrix[8] * matrix[3] * matrix[14]
        + matrix[12] * matrix[2] * matrix[11]
        - matrix[12] * matrix[3] * matrix[10];

    inv[9] = -matrix[0] * matrix[9] * matrix[15]
        + matrix[0] * matrix[11] * matrix[13]
        + matrix[8] * matrix[1] * matrix[15]
        - matrix[8] * matrix[3] * matrix[13]
        - matrix[12] * matrix[1] * matrix[11]
        + matrix[12] * matrix[3] * matrix[9];

    inv[13] = matrix[0] * matrix[9] * matrix[14]
        - matrix[0] * matrix[10] * matrix[13]
        - matrix[8] * matrix[1] * matrix[14]
        + matrix[8] * matrix[2] * matrix[13]
        + matrix[12] * matrix[1] * matrix[10]
        - matrix[12] * matrix[2] * matrix[9];

    inv[2] = matrix[1] * matrix[6] * matrix[15]
        - matrix[1] * matrix[7] * matrix[14]
        - matrix[5] * matrix[2] * matrix[15]
        + matrix[5] * matrix[3] * matrix[14]
        + matrix[13] * matrix[2] * matrix[7]
        - matrix[13] * matrix[3] * matrix[6];

    inv[6] = -matrix[0] * matrix[6] * matrix[15]
        + matrix[0] * matrix[7] * matrix[14]
        + matrix[4] * matrix[2] * matrix[15]
        - matrix[4] * matrix[3] * matrix[14]
        - matrix[12] * matrix[2] * matrix[7]
        + matrix[12] * matrix[3] * matrix[6];

    inv[10] = matrix[0] * matrix[5] * matrix[15]
        - matrix[0] * matrix[7] * matrix[13]
        - matrix[4] * matrix[1] * matrix[15]
        + matrix[4] * matrix[3] * matrix[13]
        + matrix[12] * matrix[1] * matrix[7]
        - matrix[12] * matrix[3] * matrix[5];

    inv[14] = -matrix[0] * matrix[5] * matrix[14]
        + matrix[0] * matrix[6] * matrix[13]
        + matrix[4] * matrix[1] * matrix[14]
        - matrix[4] * matrix[2] * matrix[13]
        - matrix[12] * matrix[1] * matrix[6]
        + matrix[12] * matrix[2] * matrix[5];

    inv[3] = -matrix[1] * matrix[6] * matrix[11]
        + matrix[1] * matrix[7] * matrix[10]
        + matrix[5] * matrix[2] * matrix[11]
        - matrix[5] * matrix[3] * matrix[10]
        - matrix[9] * matrix[2] * matrix[7]
        + matrix[9] * matrix[3] * matrix[6];

    inv[7] = matrix[0] * matrix[6] * matrix[11]
        - matrix[0] * matrix[7] * matrix[10]
        - matrix[4] * matrix[2] * matrix[11]
        + matrix[4] * matrix[3] * matrix[10]
        + matrix[8] * matrix[2] * matrix[7]
        - matrix[8] * matrix[3] * matrix[6];

    inv[11] = -matrix[0] * matrix[5] * matrix[11]
        + matrix[0] * matrix[7] * matrix[9]
        + matrix[4] * matrix[1] * matrix[11]
        - matrix[4] * matrix[3] * matrix[9]
        - matrix[8] * matrix[1] * matrix[7]
        + matrix[8] * matrix[3] * matrix[5];

    inv[15] = matrix[0] * matrix[5] * matrix[10]
        - matrix[0] * matrix[6] * matrix[9]
        - matrix[4] * matrix[1] * matrix[10]
        + matrix[4] * matrix[2] * matrix[9]
        + matrix[8] * matrix[1] * matrix[6]
        - matrix[8] * matrix[2] * matrix[5];

    let mut det: f32 =
        matrix[0] * inv[0] + matrix[1] * inv[4] + matrix[2] * inv[8] + matrix[3] * inv[12];

    det = 1.0 / det;

    for inv_i in inv.iter_mut() {
        *inv_i *= det;
    }
}

fn normalize(r: &mut [f32; 3]) {
    let len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    r[0] /= len;
    r[1] /= len;
    r[2] /= len;
}

fn dot(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn rotate_x(vec: &mut [f32; 3], theta: f32) {
    let (a, b) = (theta.sin(), theta.cos());
    let m = [1.0, 0.0, 0.0, 0.0, b, -a, 0.0, a, b];
    let (x, y, z) = (
        m[0] * vec[0] + m[1] * vec[1] + m[2] * vec[2],
        m[3] * vec[0] + m[4] * vec[1] + m[5] * vec[2],
        m[6] * vec[0] + m[7] * vec[1] + m[8] * vec[2],
    );
    vec[0] = x;
    vec[1] = y;
    vec[2] = z;
}

fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}
