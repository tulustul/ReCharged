// Credits (UI-17): the team, the tools, the generators and the font licence. The roll's length
// follows this list (`screens/Credits.tsx`).

export const credits: { title: string; lines: string[] }[] = [
  { title: "ReCharged", lines: ["An arcade combat racer for electric RC cars", "Inspired by Re-Volt"] },
  { title: "Created by", lines: ["Mateusz Tomczyk"] },
  { title: "Team", lines: ["A crew of orchestrated Claude AI agents", "Design · code · tracks · cars · art · music · sound · QA"] },
  {
    title: "Built with",
    lines: ["Bevy 0.20 — game engine", "bevy-react — React UI for Bevy", "Rapier (bevy_rapier3d) — physics", "Kira (bevy_kira_audio) — audio", "Rust · TypeScript · React"],
  },
  {
    title: "Generators",
    lines: [
      "Everything you see and hear is made by scripts",
      "Cars, props and rooms — Blender Python",
      "Textures — Python + numpy",
      "Music and sound effects — Python + numpy synths, ffmpeg",
      "Track previews and car thumbnails — rendered in game",
    ],
  },
  { title: "Font", lines: ["Saira — Copyright 2020 The Saira Project Authors", "github.com/Omnibus-Type/Saira", "Licensed under the SIL Open Font License 1.1"] },
  { title: "Thanks", lines: ["The Bevy and Rust communities", "Everyone who raced a toy car across a living room"] },
];

export const creditsText = { title: "Credits", thanks: "Thanks for playing!" } as const;
