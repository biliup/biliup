import { writable } from "svelte/store";

import { crossfade, fly } from "svelte/transition";


export const isLogin = writable(false);
export const template = writable({});

export const currentTemplate = writable({
    current: '',
    selectedTemplate: {
        title: '',
        files: [],
        copyright: 1,
        source: "",
        tid: 0,
        description: "",
        dynamic: "",
        tags: '',
        videos: [],
        changed: false
    }
});

export const [send, receive] = crossfade({
    duration: 800,
    fallback: (node, params) => {
        return fly(node, { x: 200, delay: 500 });
    },
});
