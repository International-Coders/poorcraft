# Upscaler Prerequisites

Upscaling is a feature, not a rescue rope.

## Required Engine Data

- render resolution separate from output resolution;
- depth texture with correct range;
- motion vectors for camera and moving objects;
- jitter control and history reset rules;
- exposure/luminance policy if the upscaler needs it;
- transparency/reactive mask policy;
- UI composition after upscaling unless explicitly supported otherwise;
- screenshot comparison at native and upscaled resolutions.

## Required Tests

- camera pan with stable objects;
- moving NPC/object vectors;
- transparent water/vegetation;
- UI sharpness and no ghosting;
- pause/menu transition;
- resize from 1280x720 to 1501x801.

Until these pass, upscaler work is marked `not_ready`.
