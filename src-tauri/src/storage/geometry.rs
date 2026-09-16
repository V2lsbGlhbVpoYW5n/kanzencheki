use super::*;
use image::{DynamicImage, GenericImageView, Rgb, RgbImage};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn validate_crop(c: &Crop) -> Result<()> {
    if [c.x, c.y, c.w, c.h].iter().any(|v| !v.is_finite())
        || c.x < 0.0
        || c.y < 0.0
        || c.w < 0.01
        || c.h < 0.01
        || c.x + c.w > 1.000001
        || c.y + c.h > 1.000001
    {
        bail!("裁切区域超出图像");
    }
    if let Some(r) = c.ratio {
        if !r.is_finite() || !(0.05..=20.0).contains(&r) {
            bail!("输出比例无效");
        }
    }
    if let Some(q) = &c.quad {
        if q.iter().any(|p| {
            !p.x.is_finite() || !p.y.is_finite() || p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0
        }) {
            bail!("四角必须位于图像内");
        }
        for i in 0..4 {
            let a = q[i];
            let b = q[(i + 1) % 4];
            let d = q[(i + 2) % 4];
            if (b.x - a.x) * (d.y - b.y) - (b.y - a.y) * (d.x - b.x) < 0.0001 {
                bail!("四角不能交叉、重叠或形成凹角");
            }
        }
    }
    Ok(())
}

pub fn rotate_crop(c: &Crop) -> Crop {
    Crop {
        x: (1.0 - c.y - c.h).max(0.0),
        y: c.x,
        w: c.h,
        h: c.w,
        quad: c.quad.map(|q| {
            [q[3], q[0], q[1], q[2]].map(|p| Point {
                x: 1.0 - p.y,
                y: p.x,
            })
        }),
        ratio: c.ratio.map(|r| 1.0 / r),
    }
}

/// Inverse homography from the output unit square to the selected quadrilateral.
fn homography(q: [Point; 4]) -> Result<[f64; 8]> {
    let mut m = [[0.0; 9]; 8];
    for (i, (u, v)) in [(0., 0.), (1., 0.), (1., 1.), (0., 1.)]
        .into_iter()
        .enumerate()
    {
        let p = q[i];
        m[2 * i] = [u, v, 1., 0., 0., 0., -p.x * u, -p.x * v, p.x];
        m[2 * i + 1] = [0., 0., 0., u, v, 1., -p.y * u, -p.y * v, p.y];
    }
    for col in 0..8 {
        let pivot = (col..8)
            .max_by(|&a, &b| m[a][col].abs().total_cmp(&m[b][col].abs()))
            .unwrap();
        m.swap(col, pivot);
        let scale = m[col][col];
        if scale.abs() < 1e-10 {
            bail!("四角无法形成有效透视变换");
        }
        for j in col..9 {
            m[col][j] /= scale;
        }
        for row in 0..8 {
            if row != col {
                let f = m[row][col];
                for j in col..9 {
                    m[row][j] -= f * m[col][j];
                }
            }
        }
    }
    Ok(std::array::from_fn(|i| m[i][8]))
}
pub fn apply_crop(image: DynamicImage, c: Option<&Crop>) -> Result<DynamicImage> {
    let Some(c) = c else { return Ok(image) };
    validate_crop(c)?;
    let (w, h) = image.dimensions();
    let Some(q) = c.quad else {
        let x = ((c.x * w as f64).floor() as u32).min(w - 1);
        let y = ((c.y * h as f64).floor() as u32).min(h - 1);
        return Ok(image.crop_imm(
            x,
            y,
            ((c.w * w as f64).floor() as u32).max(1).min(w - x),
            ((c.h * h as f64).floor() as u32).max(1).min(h - y),
        ));
    };
    let distance = |a: Point, b: Point| ((a.x - b.x) * w as f64).hypot((a.y - b.y) * h as f64);
    let width = (distance(q[0], q[1]) + distance(q[3], q[2])) / 2.;
    let height = (distance(q[0], q[3]) + distance(q[1], q[2])) / 2.;
    let ratio = c.ratio.unwrap_or(width / height);
    // Preview sources are bounded; never enlarge beyond the source's longest edge.
    let extent = width.max(height).min(w.max(h) as f64).max(2.);
    let (ow, oh) = if ratio >= 1. {
        (extent, (extent / ratio).max(2.))
    } else {
        ((extent * ratio).max(2.), extent)
    };
    let (ow, oh) = (ow.round() as u32, oh.round() as u32);
    let t = homography(q)?;
    let source = image.to_rgb8();
    let mut out = RgbImage::new(ow, oh);
    for y in 0..oh {
        for x in 0..ow {
            let u = x as f64 / (ow - 1) as f64;
            let v = y as f64 / (oh - 1) as f64;
            let d = t[6] * u + t[7] * v + 1.;
            let sx = ((t[0] * u + t[1] * v + t[2]) / d * (w - 1) as f64).clamp(0., (w - 1) as f64);
            let sy = ((t[3] * u + t[4] * v + t[5]) / d * (h - 1) as f64).clamp(0., (h - 1) as f64);
            let (ix, iy) = (sx.floor() as u32, sy.floor() as u32);
            let (fx, fy) = (sx - ix as f64, sy - iy as f64);
            let a = source.get_pixel(ix, iy);
            let b = source.get_pixel((ix + 1).min(w - 1), iy);
            let d = source.get_pixel(ix, (iy + 1).min(h - 1));
            let e = source.get_pixel((ix + 1).min(w - 1), (iy + 1).min(h - 1));
            out.put_pixel(
                x,
                y,
                Rgb(std::array::from_fn(|i| {
                    ((a[i] as f64 * (1. - fx) + b[i] as f64 * fx) * (1. - fy)
                        + (d[i] as f64 * (1. - fx) + e[i] as f64 * fx) * fy)
                        .round() as u8
                })),
            );
        }
    }
    Ok(DynamicImage::ImageRgb8(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perspective_maps_corners_and_rejects_crossing() {
        let image = RgbImage::from_fn(100, 80, |x, y| Rgb([x as u8, y as u8, 0]));
        let q = [
            Point { x: 0.1, y: 0.1 },
            Point { x: 0.9, y: 0.2 },
            Point { x: 0.8, y: 0.9 },
            Point { x: 0.2, y: 0.8 },
        ];
        let c = Crop {
            x: 0.,
            y: 0.,
            w: 1.,
            h: 1.,
            quad: Some(q),
            ratio: Some(1.),
        };
        let result = apply_crop(DynamicImage::ImageRgb8(image), Some(&c))
            .unwrap()
            .to_rgb8();
        assert_eq!(result.width(), result.height());
        assert!(result.get_pixel(0, 0)[0].abs_diff(10) <= 1);
        assert!(result.get_pixel(result.width() - 1, result.height() - 1)[0].abs_diff(79) <= 1);
        let mut bad = c.clone();
        bad.quad = Some([q[0], q[2], q[1], q[3]]);
        assert!(validate_crop(&bad).is_err());
        let mut rotated = c.clone();
        for _ in 0..4 {
            rotated = rotate_crop(&rotated);
        }
        for (a, b) in rotated.quad.unwrap().iter().zip(q) {
            assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9);
        }
    }
}
