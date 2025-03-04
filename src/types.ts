export interface AudioDeviceInfo {
  id: string;
  name: string;
  volume: number;
  muted: boolean;
}

export interface WindowsAudioState {
  default: string;
  audioDeviceList: AudioDeviceInfo[];
}


const eventNames = [
  "DefaultDeviceChanged",
  "DeviceAdded",
  "DeviceRemoved",
  "DeviceStateChanged",
  "PropertyValueChanged",
  "VolumeChanged",
] as const;

export type EventName = typeof eventNames[number];


interface EventPayloadBase {
  type: EventName;
  id: string;
}

export interface DefaultDeviceChanged extends EventPayloadBase { }
export interface DeviceAdded extends EventPayloadBase { }
export interface DeviceRemoved extends EventPayloadBase { }
export interface DeviceStateChanged extends EventPayloadBase {
  state: number;
}
export interface PropertyValueChanged extends EventPayloadBase {
  key: string;
}
export interface VolumeChanged extends EventPayloadBase {
  volume: number;
  muted: boolean;
}

export type Notify = | DefaultDeviceChanged | DeviceAdded | DeviceRemoved | DeviceStateChanged | PropertyValueChanged | VolumeChanged;



export interface AudioStateChangePayload {
  windowsAudioState: WindowsAudioState;
  notification?: Notify;
}



export interface MeterProps {
  device: AudioDeviceInfo;
  defaultVolume?: number;
  deviceList?: AudioDeviceInfo[];
}

export interface AppContextValue {
  addIgnoreDragTarget: (target: HTMLElement) => void;
  removeIgnoreDragTarget: (target: HTMLElement) => void;
}
