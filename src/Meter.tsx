import VolumeMuteIcon from '@mui/icons-material/VolumeMute';
import VolumeOffIcon from '@mui/icons-material/VolumeOff';
import VolumeUpIcon from '@mui/icons-material/VolumeUp';
import { Grid, IconButton, Slider, Stack, Typography } from "@mui/material";
import { UnlistenFn, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/tauri";
import { useCallback, useContext, useEffect, useRef, useState } from "react";
import AppContext from "./AppContext";
import { QueryKind, invokeQuery } from "./ipc";
import { MeterProps } from "./types";

const volumeStep = 0.01;

async function registerListeners() {
  const DefaultAudioChange = listen('DefaultAudioChange', (event) => {
    invokeQuery({
      kind: "DefaultAudioChange",
      id: event.payload as string,
    });
  });

  await Promise.all([
    DefaultAudioChange,
  ]);
}

registerListeners();

export default function Meter(props: MeterProps) {

  const appContext = useContext(AppContext);
  const buttonRef = useRef<HTMLButtonElement | null>(null);
  const sliderRef = useRef<HTMLSpanElement | null>(null);

  useEffect(() => {

    let unListen: UnlistenFn | undefined = undefined;

    (async () => {
      const unListenQuit = await listen("quit", () => invoke("quit"));

      unListen = () => {
        unListenQuit();
      }

    })();

    return () => {
      unListen && unListen();
    }

  }, [])

  useEffect(() => {
    const { addIgnoreDragTarget, removeIgnoreDragTarget } = appContext;

    buttonRef.current && addIgnoreDragTarget(buttonRef.current);
    sliderRef.current && addIgnoreDragTarget(sliderRef.current);

    return () => {
      buttonRef.current && removeIgnoreDragTarget(buttonRef.current);
      sliderRef.current && removeIgnoreDragTarget(sliderRef.current);
    }

  }, [appContext])

  const { device, defaultVolume, deviceList } = props;

  const [volume, setVolume] = useState(device.volume || 0);
  const [muted, setMuted] = useState(device.muted);

  useEffect(() => setVolume(defaultVolume || 0), [defaultVolume]);
  useEffect(() => setMuted(device.muted), [device.muted]);

  const handlerIdRef = useRef<number | null>(null);
  const invokeChangeVolume = useCallback(async (volume: number) => {
    if (!device) {
      return;
    }

    if (handlerIdRef.current !== null) {
      clearTimeout(handlerIdRef.current);
    }

    handlerIdRef.current = window.setTimeout(async () => {
      await invokeQuery({
        kind: "VolumeChange",
        id: device.id,
        volume,
      });
    }, 10);

  }, [device.id]);

  const handleChangeVolume = useCallback((event: Event, volume: number | number[]) => {

    event.stopPropagation();
    event.preventDefault();

    setVolume(volume as number);
    invokeChangeVolume(volume as number);

  }, [invokeChangeVolume])

  const handleWheel = useCallback((event: WheelEvent) => {

    if (!device) {
      return;
    }

    if (muted) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();

    setVolume((volume) => {

      const delta = event.deltaY || event.deltaX;

      const direction = volume + (delta > 0 ? -volumeStep : volumeStep);
      const nextVolume = Math.min(1, Math.max(0, direction));

      invokeChangeVolume(nextVolume);

      return nextVolume;
    })


  }, [invokeChangeVolume, muted]);

  const scrollAreaRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!scrollAreaRef.current) {
      return;
    }

    scrollAreaRef.current.addEventListener("wheel", handleWheel);

    return () => {
      scrollAreaRef.current?.removeEventListener("wheel", handleWheel);
    }
  }, [handleWheel]);


  const handleToggleMute = useCallback(async () => {

    if (!device) {
      return;
    }

    setMuted(!muted);

    await invokeQuery({
      kind: "MuteStateChange",
      id: device.id,
      muted: !muted,
    });

  }, [device, muted]);



  const handleContextMenu = useCallback((e: MouseEvent) => {

    if (!deviceList) {
      return;
    }

    e.preventDefault();

    const event: QueryKind = "DefaultAudioChange";

    const items = deviceList.map((d) => ({
      label: d.name,
      event,
      payload: d.id,
      checked: d.id === device.id,
    }));

    invoke("plugin:context_menu|show_context_menu", {
      pos: { x: e.clientX, y: e.clientY },
      items: [
        ...items,
        { is_separator: true },
        { label: "Quit", event: "quit" }
      ]
    });


  }, [device, deviceList]);

  useEffect(() => {
    window.addEventListener("contextmenu", handleContextMenu);

    return () => {
      window.removeEventListener("contextmenu", handleContextMenu);
    }
  }, [handleContextMenu]);


  const displayVolume = useCallback((v: number) => Math.round(v * 100), []);

  return (
    <Grid
      container
      display="grid"
      gridTemplateColumns={"max-content 1fr"}
      gridTemplateRows={"repeat(2, auto)"}
      alignItems="center"
      ref={scrollAreaRef}
    >
      <IconButton
        onMouseDown={(e) => e.stopPropagation()}
        onClick={handleToggleMute}
        size="small"
        ref={buttonRef}
      >
        {muted ? <VolumeOffIcon /> : volume === 0 ? <VolumeMuteIcon /> : <VolumeUpIcon />}
      </IconButton>

      <Typography
        variant="body1"
        component="div"
        width="100%"
        noWrap
      >
        {device.name}
      </Typography>

      <div></div>

      <Stack
        direction="row"
        alignItems="center"
        spacing={2}
      >
        <Slider
          value={volume}
          onMouseDown={(e) => e.stopPropagation()}
          onChange={handleChangeVolume}
          min={0}
          max={1}
          step={volumeStep}
          disabled={muted}
          size="small"
          ref={sliderRef}
        />
        <Typography variant="body1" textAlign="center" width="2em">
          {displayVolume(volume)}
        </Typography>
      </Stack>

    </Grid>
  )
}
