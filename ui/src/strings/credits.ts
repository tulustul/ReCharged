// Credits (UI-17): team, tools, licences.

export const credits: { title: string; lines: string[] }[] = [
  { title: "ReCharged", lines: ["An arcade combat racer for electric RC cars", "Inspired by Re-Volt"] },
  { title: "Design & Development", lines: ["Mateusz Tomczyk", "with a crew of Claude agents"] },
  { title: "Engine", lines: ["Bevy 0.20", "bevy-react (React UI for Bevy)", "Rapier physics (bevy_rapier3d)", "Kira audio (bevy_kira_audio)"] },
  { title: "Art, Music & Sound", lines: ["Cars modelled with Blender scripts", "Music composed and rendered procedurally", "Sound effects synthesised from scratch"] },
  { title: "Fonts", lines: ["Saira — The Saira Project Authors (Omnibus-Type)", "SIL Open Font License 1.1"] },
  { title: "Thanks", lines: ["The Bevy and Rust communities", "Everyone who raced a toy car across a living room"] },
];

export const creditsText = { title: "Credits", thanks: "Thanks for playing!" } as const;
