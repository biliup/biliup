import React, {useEffect} from "react";
import Player from 'xgplayer';
import Artplayer from 'artplayer';
import 'xgplayer/dist/index.min.css';
import FlvPlugin from "xgplayer-flv";
import FlvJsPlugin from 'xgplayer-flv.js'
import Mp4Plugin from "xgplayer-mp4";

const Players: React.FC<{url: string}> = ({url}) => {
    useEffect(() => {
        // let player: Player | null = new Player({
        //     id: 'mse',
        //     url: url,
        //     height: '100%',
        //     plugins: [FlvPlugin, Mp4Plugin],
        //     // plugins: [FlvJsPlugin],
        //     width: '100%',
        // });
        // return () => {
        //     player?.destroy();
        //     player = null;
        // };
        let player: Artplayer | null = new Artplayer({
            container: '.artplayer-app',
            url: url,
            autoSize: true,
            fullscreen: true,
            fullscreenWeb: true,
            autoOrientation: true,
            plugins: [],
        });
        return () => {
            player?.destroy();
            player = null;
        };
    }, [url])
    return  <div className="artplayer-app" style={{width: '100%', height: '100%'}}></div>;
}

export default Players;