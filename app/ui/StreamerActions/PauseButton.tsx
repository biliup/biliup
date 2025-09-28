import React, { useState } from 'react';
import { useSWRConfig } from 'swr';
import useSWRMutation from 'swr/mutation';
import {Button} from "@douyinfe/semi-ui";
import {IconPause, IconPlay} from "@douyinfe/semi-icons";
import {API_BASE, LiveStreamerEntity} from "@/app/lib/api-streamer";

interface PauseButtonProps {
    streamer: LiveStreamerEntity;
    onSuccess?: () => void;
    onError?: (error: Error) => void;
}

// 暂停主播
export const pauseStreamer = async (url: string,  ) => {
    const response = await fetch(API_BASE + url,
        {
            method: 'PUT',
            // headers: {'Content-Type': 'application/json'},
        }
);
    return response;
};

export const PauseButton: React.FC<PauseButtonProps> = ({
                                                            streamer,
                                                            onSuccess,
                                                            onError
                                                        }) => {
    const { mutate } = useSWRConfig();

    const { trigger: pauseTrigger } = useSWRMutation(
        `/v1/streamers/${streamer.id}/pause`,
        pauseStreamer
    );

    const handlePause = async () => {
        try {
            await pauseTrigger();
            // 重新加载列表数据
            await mutate('/v1/streamers');
            onSuccess?.();
        } catch (error) {
            console.error('暂停失败:', error);
            onError?.(error as Error);
        }
    };
    console.log("????????????", streamer.status)
    return (
        <Button onClick={handlePause} icon={streamer.status === 'Pause'? <IconPlay />: <IconPause />} theme="borderless" aria-label="暂停" />
    );
};