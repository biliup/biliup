import React, { useEffect, useRef } from "react";
import Player from 'xgplayer';
import Artplayer from 'artplayer';
import 'xgplayer/dist/index.min.css';
import FlvPlugin from "xgplayer-flv";
import FlvJsPlugin from "xgplayer-flv.js";

type VideoPlayer = Player | Artplayer | null;

interface PlayerConfig {
  url: string;
  height?: string;
  width?: string;
}

const Players: React.FC<PlayerConfig> = ({ url, height = '100%', width = '100%' }) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const playerRef = useRef<VideoPlayer>(null);

    useEffect(() => {
        if (!containerRef.current) return;

        try {
            if (playerRef.current) {
                playerRef.current.destroy();
                playerRef.current = null;
            }

            if (url.endsWith('.flv')) {
                playerRef.current = new Player({
                    el: containerRef.current,
                    url,
                    height,
                    width,
                    plugins: [FlvPlugin],
                });
            } else {
                playerRef.current = new Artplayer({
                    container: containerRef.current,
                    url,
                    autoSize: true,
                    fullscreen: true,
                    fullscreenWeb: true,
                    autoOrientation: true,
                    plugins: [],
                });
            }
        } catch (error) {
            console.error('播放器初始化失败:', error);
        }

        return () => {
            if (playerRef.current) {
                playerRef.current.destroy();
                playerRef.current = null;
            }
        };
    }, [url, height, width]);

    return (
        <div
            ref={containerRef}
            style={{ width, height }}
        />
    );
}

export default Players;