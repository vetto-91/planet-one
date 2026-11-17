// Estensione del materiale standard (ADR 0061), usata in due modi:
// - mare (params.w = 0): l'alfa cresce con la distanza orizzontale dalla camera, da quella del
//   materiale a params.x metri fino a 1 a params.y metri, così il fondale si vede solo da vicino;
// - dischi di sole e luna (params.w = 1): niente sotto l'orizzonte del mare (quota params.x,
//   mare visibile fino a params.y metri), così tramontano dietro il mare e non ci passano davanti.
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> params: vec4<f32>;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let to_frag = in.world_position.xyz - view.world_position;
    let across = length(to_frag.xz);

    if params.w > 0.5 {
        // Orizzonte visibile: il bordo del mare a params.y metri, visto dall'altezza della camera.
        let dip = max(view.world_position.y - params.x, 0.0) / params.y;
        if to_frag.y < -across * dip {
            discard;
        }
    } else {
        let t = smoothstep(params.x, params.y, across);
        pbr_input.material.base_color.a = mix(pbr_input.material.base_color.a, 1.0, t);
        // Lontano le onde sono più piccole di un pixel e la loro piastrella si ripeterebbe a vista:
        // la normale sfuma verso l'alto.
        let far = smoothstep(30.0, 120.0, across);
        pbr_input.N = normalize(mix(pbr_input.N, vec3<f32>(0.0, 1.0, 0.0), far));
    }

    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color,
    );

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
