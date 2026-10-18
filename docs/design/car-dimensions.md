# Car dimensions (single source of truth)

Owned by the orchestrator. Used by **rc_vehicle** (`assets/data/vehicles/<car>.ron` wheel and
chassis geometry) and by the **car models** (`assets/models/cars/<car>.glb` + `<car>.wheels.ron`).
A test in rc_vehicle checks that the physics wheel layout matches each model's `.wheels.ron`.

Units: world units (1 u = 22.5 cm). Bevy axes: +Y up, **−Z forward**, +X right. The car origin
is the **ground-contact centre**: halfway between the axles, at ground level when the car rests
at ride height. Front axle at `z = −wheelbase/2`, rear axle at `z = +wheelbase/2`; wheel centres
at `y = wheel radius`, `x = ±track/2`.

(Blender: model facing +Y, so the front axle sits at `y = +wheelbase/2`; export "+Y Up".)

| Car | Body | Length | Width | Height | Wheelbase | Track F / R | Wheel radius F / R | Wheel width F / R | Ground clearance |
|---|---|---|---|---|---|---|---|---|---|
| sparky | buggy, roll cage | 2.00 | 1.25 | 0.75 | 1.30 | 1.10 / 1.10 | 0.30 / 0.32 | 0.22 / 0.25 | 0.22 |
| dumpster | monster truck | 2.20 | 1.45 | 1.10 | 1.35 | 1.25 / 1.25 | 0.42 / 0.42 | 0.32 / 0.32 | 0.40 |
| zip | mini touring | 1.90 | 1.00 | 0.50 | 1.20 | 0.88 / 0.88 | 0.20 / 0.20 | 0.16 / 0.18 | 0.08 |
| torque | muscle car | 2.15 | 1.15 | 0.60 | 1.35 | 0.98 / 1.02 | 0.24 / 0.26 | 0.18 / 0.26 | 0.10 |
| hornet | rally hatch | 1.85 | 1.10 | 0.70 | 1.20 | 0.96 / 0.96 | 0.24 / 0.24 | 0.18 / 0.18 | 0.16 |
| phantom | sports coupe, wing | 2.05 | 1.12 | 0.50 (wing to 0.56) | 1.30 | 0.98 / 1.00 | 0.22 / 0.23 | 0.18 / 0.22 | 0.08 |
| volt_gt | hypercar | 2.10 | 1.15 | 0.46 | 1.32 | 1.00 / 1.02 | 0.22 / 0.23 | 0.19 / 0.24 | 0.07 |
| bolt | open-wheel formula | 2.20 | 1.15 | 0.42 | 1.40 | 1.04 / 1.06 | 0.22 / 0.24 | 0.18 / 0.26 | 0.06 |

- Length / Width / Height are the **body's** bounding box **excluding wheels and antenna** (the
  physics chassis collider lives inside it). Overall width = max(track) + wheel width; wheels of
  open-wheel cars (bolt, volt_gt rear) may rise above the body height.
- Ground clearance = lowest chassis point above the ground at rest.
- The physics may use a simplified collider (box/convex hull) inside the body envelope; wheels are
  ray/shape casts, not colliders.

`<car>.wheels.ron` written by the model generator:
```ron
(
    car: "sparky",
    wheels: [ // order: fl, fr, rl, rr; centre in car space (Bevy axes), radius, width
        (name: "wheel_fl", center: (-0.55, 0.30, -0.65), radius: 0.30, width: 0.22),
        (name: "wheel_fr", center: ( 0.55, 0.30, -0.65), radius: 0.30, width: 0.22),
        (name: "wheel_rl", center: (-0.55, 0.32,  0.65), radius: 0.32, width: 0.25),
        (name: "wheel_rr", center: ( 0.55, 0.32,  0.65), radius: 0.32, width: 0.25),
    ],
)
```
