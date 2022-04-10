import tippy, {animateFill} from "tippy.js";
// import 'tippy.js/dist/tippy.css'; // optional for styling
// import 'tippy.js/dist/backdrop.css';
import 'tippy.js/animations/shift-away.css';
import 'tippy.js/themes/light.css';
import Partition from "./Partition.svelte";
import {writable} from "svelte/store";
import Pop from "./Pop.svelte";
import {check_outros, group_outros, transition_out} from "svelte/internal";

export let partition = writable(null);

export function archivePre(node, combine) {
    let off;
    let detail;
    let partition;
    tippy(node, {
        // content: `1313`,
        arrow: false,
        trigger: 'click',
        allowHTML: true,
        theme: 'light',
        placement: 'bottom-start',
        animateFill: true,
        plugins: [animateFill],
        inertia: true,
        interactive: true,
        onCreate(instance) {

            partition = new Partition({
                target: <Element>instance.popper.firstChild.lastChild,
                props: {
                    current: combine.current,
                    currentChildren: combine.currentChildren
                }
            });
            off = partition.$on('tid', event => {
                combine.callback(event.detail.tid, event.detail.parent, event.detail.children);
                instance.hide();
                console.log(event);
                detail = event.detail;
            });
        },
        onShown(instance) {
            // @ts-ignore
            instance.popper.firstChild.lastChild.firstChild.firstChild.scrollTo({
                top: detail?.scroll[0]?.offsetTop - 3,
                // left: 100,
                behavior: 'smooth'
            });
            // @ts-ignore
            instance.popper.firstChild.lastChild.firstChild.lastChild.scrollTo({
                top: detail?.scroll[1]?.offsetTop - 8,
                // left: 100,
                behavior: 'smooth'
            });
            // console.log(instance.popper.firstChild.lastChild.firstChild.firstChild.scrollTop);
        },
        onDestroy(instance) {
            off();
        },
    });
    return {
        update(combine) {
            // partition = newDuration;
            partition.$set({
                current: combine.current,
                currentChildren: combine.currentChildren
            });
        },
    };
}

const notificationHistory = [];
export const notifyHistory = writable(notificationHistory);

export function createPop(msg, duration = 3000, mode = 'Error') {
    notificationHistory.push({
        type: mode,
        msg: msg,
        date: new Date(),
    });
    notifyHistory.set(notificationHistory);
    const pop = new Pop({
        target: document.querySelector('#alerts'),
        intro: true,
        props: {
            msg: msg,
            mode: mode
        }
    });
    setTimeout(() => outroAndDestroy(pop), duration);
}


// Workaround for https://github.com/sveltejs/svelte/issues/4056
const outroAndDestroy = (instance) => {
    if (instance.$$.fragment && instance.$$.fragment.o) {
        group_outros();
        transition_out(instance.$$.fragment, 0, 0, () => {
            instance.$destroy();
        });
        check_outros();
    } else {
        instance.$destroy();
    }
};
