import { invoke } from "@tauri-apps/api/tauri";

export type AudioDict = {
  kind: "AudioDict";
};

export type DefaultAudioChange = {
  kind: "DefaultAudioChange";
  id: string;
};

export type VolumeChange = {
  kind: "VolumeChange";
  id: string;
  volume: number;
};

export type MuteStateChange = {
  kind: "MuteStateChange";
  id: string;
  muted: boolean;
};

export type Channels = {
  kind: "Channels";
};


export type Query = AudioDict | DefaultAudioChange | VolumeChange | MuteStateChange | Channels;

export type QueryKind = Query["kind"];

export async function invokeQuery(query: Query): Promise<void> {
  await invoke("query", { query });
}
