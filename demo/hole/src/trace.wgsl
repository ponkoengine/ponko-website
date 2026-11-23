// The whole picture: every pixel's light traced back from the camera past
// a Schwarzschild black hole. Units: the hole's Schwarzschild radius is 1
// and it sits at the origin.
//
// Each ray is a photon path, integrated step by step with the usual
// trick: an acceleration of -1.5 h² pos / r⁵ (h = |pos × vel|) bends a
// straight-line path exactly as the hole's geometry does. Rays reaching
// r < 1 are swallowed. Every pass through the disk plane adds the disk's
// light there, front to back, so the far side shows over the top, its
// second image under the bottom, and the photon ring comes from rays that
// circle the hole before they escape. Escaping rays see the sky.
//
// The disk's gas is the simulation: the game splats its particles into a
// map of the disk (custom_texture0, angle across, sqrt(radius) down):
//   r  gas density, relative to the average at that radius (1 average),
//      stored divided by 4;
//   g  a thrown star's debris: its stream, and the heat it leaves in the disk.
//
// custom[0]: x step size as a fraction of r, y seconds, z flare,
//   w 2 tan(vertical field of view / 2) (with the frame's height: a pixel's angle).
// custom[1..3]: thrown stars: xyz position, w brightness (0: none).

const TAU: f32 = 6.2831853;
const MAP_R0: f32 = 1.2;
const MAP_R1: f32 = 17.0;
const DISK_IN: f32 = 3.0;
const DISK_OUT: f32 = 16.0;
const MAX_STEPS: i32 = 220;

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3<f32>(0.1031, 0.1030, 0.0973));
    let r = q + dot(q, q.yxz + 33.33);
    return fract((r.x + r.y) * r.z);
}

// Cell centers on the six faces of a cube around the sky. The compact
// support ends inside each cell, so cell edges never show up.
fn star_layer(face: vec3<f32>, cells: f32, keep: f32, footprint: f32) -> vec3<f32> {
    let g = (face.xy * 0.5 + 0.5) * cells;
    let c = vec3<f32>(floor(g), face.z);
    let h = hash3(c);
    if h < keep {
        return vec3<f32>(0.0);
    }
    let at = vec2<f32>(hash3(c + 17.0), hash3(c + 41.0)) * 0.28 + 0.36;
    let off = length(fract(g) - at);
    let strength = (h - keep) / (1.0 - keep);
    let pixel = footprint * cells * 0.65;
    let core = mix(0.026, 0.068, strength);
    let sigma = sqrt(core * core + pixel * pixel * 0.18);
    let resolved = 1.0 - smoothstep(0.19, 0.42, pixel);
    let glow = exp(-0.5 * pow(off / sigma, 2.0)) * (1.0 - smoothstep(0.22, 0.35, off)) *
        (0.17 + 2.2 * pow(strength, 4.0)) * resolved * (core * core / (sigma * sigma));
    let tint = mix(vec3<f32>(1.0, 0.78, 0.6), vec3<f32>(0.7, 0.82, 1.0), hash3(c + 5.0));
    return tint * glow;
}

fn cube_face(d: vec3<f32>) -> vec3<f32> {
    let axis = abs(d);
    if axis.x >= max(axis.y, axis.z) {
        return vec3<f32>(d.yz / axis.x, select(1.0, 2.0, d.x > 0.0));
    }
    if axis.y >= axis.z {
        return vec3<f32>(d.xz / axis.y, select(3.0, 4.0, d.y > 0.0));
    }
    return vec3<f32>(d.xy / axis.z, select(5.0, 6.0, d.z > 0.0));
}

fn sky(d: vec3<f32>, footprint: f32, stars: f32) -> vec3<f32> {
    // A faint band of the galaxy across the sky, and its stars.
    let band_axis = normalize(vec3<f32>(0.3, 0.9, -0.35));
    let lat = dot(d, band_axis);
    let lon = atan2(d.z, d.x);
    let dust = fbm2(vec2<f32>(lon * 3.0, lat * 9.0));
    let band = exp(-lat * lat * 14.0) * (0.35 + 0.65 * dust);
    var c = vec3<f32>(0.010, 0.012, 0.022) * band * 0.45;
    c += vec3<f32>(0.016, 0.010, 0.008) * exp(-lat * lat * 60.0) * dust * dust * 0.5;
    let face = cube_face(d);
    c += star_layer(face, 72.0, 0.955, footprint) * (0.7 * stars);
    c += star_layer(face, 188.0, 0.92, footprint) * ((0.08 + band * 0.35) * stars);
    return c;
}

// Colour of hot gas by temperature (1: the inner edge, as seen at rest):
// deep red, orange, gold, white-yellow, blue-white.
fn heat_color(t: f32) -> vec3<f32> {
    let a = vec3<f32>(0.42, 0.03, 0.005);
    let b = vec3<f32>(1.0, 0.24, 0.03);
    let c = vec3<f32>(1.0, 0.55, 0.16);
    let d = vec3<f32>(1.0, 0.8, 0.5);
    let e = vec3<f32>(0.82, 0.88, 1.0);
    if t < 0.35 { return mix(a, b, smoothstep(0.1, 0.35, t)); }
    if t < 0.6 { return mix(b, c, smoothstep(0.35, 0.6, t)); }
    if t < 0.95 { return mix(c, d, smoothstep(0.6, 0.95, t)); }
    return mix(d, e, smoothstep(0.95, 1.7, t));
}

fn erf(x: f32) -> f32 {
    // Winitzki's approximation: good to a few parts in a thousand.
    let x2 = x * x;
    let a = 0.147;
    let v = sqrt(1.0 - exp(-x2 * (1.2732395 + a * x2) / (1.0 + a * x2)));
    return select(-v, v, x >= 0.0);
}

// The disk's half thickness at radius r.
fn thickness(r: f32) -> f32 {
    return 0.03 + 0.008 * r;
}

// Fine streaks the shear draws out of the gas, turning with it. Two
// layers half a period apart fade in turn, so the winding never piles up.
fn streaks(r: f32, angle: f32, t: f32) -> f32 {
    let spin = 10.0 * sqrt(r * 0.5) / max(r - 1.0, 0.5) / r;
    let period = 7.0;
    let phase = t / period;
    var sum = 0.0;
    for (var k = 0; k < 2; k++) {
        let f = fract(phase + f32(k) * 0.5);
        let w = 1.0 - abs(2.0 * f - 1.0);
        let a = angle - spin * f * period + f32(k) * 2.1;
        // Seamless around the circle: blend across the wrap.
        let u = fract(a / TAU);
        let q = vec2<f32>(log(r) * 26.0, u * 9.0);
        let q2 = vec2<f32>(log(r) * 26.0, (u - 1.0) * 9.0);
        let n = mix(noise2(q * vec2<f32>(1.0, 1.0) + f32(k) * 7.0), noise2(q2 + f32(k) * 7.0), smoothstep(0.85, 1.0, u));
        let n2 = noise2(vec2<f32>(log(r) * 44.0, u * 23.0 + f32(k) * 3.0));
        sum += w * (0.62 * n + 0.38 * n2);
    }
    return 0.3 + 1.4 * sum;
}

// The disk at p: rgb its surface brightness (what an opaque patch would
// show), a its opacity per unit of column. `back` points toward the camera.
fn disk_at(p: vec3<f32>, back: vec3<f32>) -> vec4<f32> {
    let r = length(p.xz);
    if r < MAP_R0 || r > MAP_R1 {
        return vec4<f32>(0.0);
    }
    let angle = atan2(p.z, p.x);
    let s0 = sqrt(MAP_R0);
    let v = (sqrt(r) - s0) / (sqrt(MAP_R1) - s0);
    let m = textureSampleLevel(custom_texture0, material_sampler, vec2<f32>(angle / TAU + 0.5, v), 0.0);
    let gas = m.r * 4.0;
    let debris = m.g * m.g * 6.0;

    // Temperature: hottest at the inner edge, falling outward; the gas
    // falling in inside the last stable orbit stays hot but thins out.
    let t_rest = pow(DISK_IN / max(r, DISK_IN), 0.75);
    let plunge = smoothstep(1.4, DISK_IN + 0.3, r);
    let edge = 1.0 - smoothstep(DISK_OUT - 6.0, DISK_OUT + 0.5, r);
    let fine = mix(1.0, streaks(r, angle, material.custom[0].y), smoothstep(2.6, 4.5, r) * 0.9);
    let lumps = mix(1.0, pow(max(gas, 0.0), 1.35) * fine, plunge);
    let amount = mix(0.35, 1.0, edge) * edge;

    // Doppler: the gas orbits, a little under light speed near the hole.
    let tangent = vec3<f32>(-p.z, 0.0, p.x) / r;
    let beta = min(sqrt(0.5 / max(r - 1.0, 0.4)), 0.62);
    let gamma = inverseSqrt(1.0 - beta * beta);
    let doppler = 1.0 / (gamma * (1.0 - beta * dot(tangent, back)));
    let g = doppler * sqrt(max(1.0 - 1.0 / max(r, 1.0), 0.05));
    let beam = g * g * g;

    let flare = material.custom[0].z * exp(-(r - 2.0) * 0.45);
    let t_seen = t_rest * g * (1.0 + 0.2 * flare);
    let falloff = pow(DISK_IN / max(r, DISK_IN), 3.0);
    var light = heat_color(t_seen) * (4.2 * falloff * beam * (1.0 + flare) * (0.3 + 0.7 * lumps) * mix(0.5, 1.0, plunge));
    var opacity = (0.2 + 0.8 * lumps) * amount * mix(0.7, 2.5, smoothstep(DISK_OUT - 2.0, DISK_IN + 1.5, r)) * mix(0.1, 1.0, plunge * plunge);
    // A star's debris: white hot, over the gas.
    let hot = min(debris, 4.0);
    let debris_opacity = hot * 1.5;
    light = (light * opacity + mix(vec3<f32>(1.0, 0.7, 0.4), vec3<f32>(1.0, 0.93, 0.8), min(hot, 1.0)) * 5.0 * beam * debris_opacity)
        / max(opacity + debris_opacity, 1e-4);
    opacity += debris_opacity;
    return vec4<f32>(light, opacity);
}

// Glow of a thrown star along a straight piece of the ray from a to b:
// a bright core and a soft halo, each a Gaussian ball integrated along the
// piece (so the glow does not depend on how the ray was cut into steps).
fn star_glow(a: vec3<f32>, b: vec3<f32>, star: vec4<f32>) -> vec3<f32> {
    let seg = b - a;
    let len = length(seg);
    let dir = seg / max(len, 1e-5);
    let t0 = dot(star.xyz - a, dir);
    let d2 = dot(star.xyz - a, star.xyz - a) - t0 * t0;
    var glow = vec3<f32>(0.0);
    for (var k = 0; k < 2; k++) {
        let s = select(0.7, 0.2, k == 0);
        let weight = select(0.16, 1.0, k == 0);
        let along = 0.8862269 * s * (erf((len - t0) / s) - erf(-t0 / s));
        let tint = select(vec3<f32>(0.55, 0.72, 1.0), vec3<f32>(1.0, 0.97, 0.92), k == 0);
        glow += tint * (exp(-d2 / (s * s)) * along * weight / s);
    }
    return glow * star.w;
}

fn trace(origin: vec3<f32>, dir0: vec3<f32>) -> vec3<f32> {
    var pos = origin;
    var vel = dir0;
    let hv = cross(pos, vel);
    let h2 = dot(hv, hv);
    let quality = material.custom[0].x;
    let leave = max(length(origin) + 1.0, 32.0);
    var light = vec3<f32>(0.0);
    var through = 1.0;
    var nearest = 1e9;
    var swallowed = false;
    var escaped = false;
    for (var i = 0; i < MAX_STEPS; i++) {
        let r2 = dot(pos, pos);
        let r = sqrt(r2);
        nearest = min(nearest, r);
        if r < 1.0 {
            swallowed = true;
            break;
        }
        if r > leave && dot(pos, vel) > 0.0 {
            escaped = true;
            break;
        }
        // Shorter steps near the photon sphere, where paths are most sensitive.
        let dt = quality * r * clamp(r - 0.9, 0.4, 1.0);
        let prev = pos;
        pos += vel * dt;
        let p2 = dot(pos, pos);
        vel += -1.5 * h2 * pos / (p2 * p2 * sqrt(p2)) * dt;

        // The disk, where this piece of the ray comes near the plane.
        let ya = prev.y;
        let yb = pos.y;
        let hmax = thickness(MAP_R1) * 3.0;
        if (ya * yb <= 0.0 || min(abs(ya), abs(yb)) < hmax) && min(length(prev.xz), length(pos.xz)) < MAP_R1 + 1.0 && through > 0.004 {
            // The point of the piece nearest the plane.
            var f = 0.5;
            if ya * yb <= 0.0 {
                f = ya / (ya - yb);
            } else if abs(yb) < abs(ya) {
                f = 1.0;
            } else {
                f = 0.0;
            }
            let at = mix(prev, pos, f);
            let rr = length(at.xz);
            let hgt = thickness(rr);
            let sig = hgt * 1.4142;
            let seg = length(pos - prev);
            var column = 0.0;
            if abs(yb - ya) > 1e-4 {
                column = seg / (yb - ya) * 0.8862269 * sig * (erf(yb / sig) - erf(ya / sig));
            } else {
                column = seg * exp(-(ya / sig) * (ya / sig));
            }
            // In units of the disk's column seen face on.
            column = column / (hgt * 2.5066);
            if column > 1e-4 {
                let back = -normalize(pos - prev);
                let d = disk_at(at, back);
                let kept = exp(-d.a * column);
                light += through * d.rgb * (1.0 - kept);
                through *= kept;
            }
        }
        for (var s = 1; s < 4; s++) {
            let star = material.custom[s];
            if star.w > 0.0 {
                light += through * star_glow(prev, pos, star);
            }
        }
        if through < 0.004 {
            break;
        }
    }
    if escaped || !swallowed {
        let stars = smoothstep(2.0, 4.0, nearest);
        let footprint = material.custom[0].w / frame.screen.y;
        light += through * sky(normalize(vel), footprint, select(stars * 0.3, stars, escaped));
    }
    return light;
}

fn effect(p: Pixel) -> vec3<f32> {
    let ndc = vec2<f32>(p.uv.x * 2.0 - 1.0, 1.0 - p.uv.y * 2.0);
    let a = frame.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let b = frame.inv_view_proj * vec4<f32>(ndc, 0.5, 1.0);
    let origin = frame.camera.xyz;
    let dir = normalize(b.xyz / b.w - a.xyz / a.w);
    return trace(origin, dir);
}
