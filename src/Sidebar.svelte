<script lang="ts">
    import {currentTemplate, send, template} from "./store.ts";
    import {fly} from 'svelte/transition';
    import {flip} from 'svelte/animate';
    import Modal from "./Modal.svelte";

    let face = 'noface.jpg';
    let name = null;


    export let items = [];

    async function add() {
        let name = '未命名模板' + Object.keys($template).length;
        $template[name] = {
            title: '',
            url: '',
            copyright: 1,
            source: "",
            tid: 171,
            description: "",
            tags: '',
            dynamic: "",
            cover: '',
            desc_format_id: 0,
            atomicInt: 0
        };
        let res;
        res = await fetch('/api/getconfig')
        let res_json = await res.json();
        res_json.streamers = $template;
        await fetch('/api/setconfig',{
            method: "POST",
            body:JSON.stringify(res_json)
        })
        select(name);

    }

    function select(item) {
        $currentTemplate.selectedTemplate = $template[item];
        $currentTemplate.current = item;
    }


    let lines = ['ws', 'qn', 'auto', 'bda2', 'kodo','cos'];
    let is_toml = false;
    let User={
        SESSDATA:"",
        bili_jct:"",
        DedeUserID__ckMd5:"",
        DedeUserID:"",
        access_token:""

    }

    let line = 'auto';
    let limit = 3;

    async function loadSettings() {
        let ret = await fetch('/api/basic');
        let ret_json = await ret.json();
        console.log(ret_json);
        ret_json.line = ret_json.line.toLowerCase();
        if (ret_json.line === null) {
            line = 'auto';
        } else {
            line = ret_json.line;
        }
        is_toml = ret_json.toml;
        User=ret_json.user;
        limit = <number>ret_json['limit'];
    }

    async function saveSettings() {
        let ret = await fetch('/api/basic');
        let ret_json = await ret.json();
        console.log(ret_json);
        if (ret_json.line === null) {
            ret_json.line = 'auto';
        } else {
            ret_json.line = line
        }
        ret_json.limit = limit;
        ret_json.user=User;
        console.log(User);
        await fetch('/api/setbasic',{
            method: "POST",
            body:JSON.stringify(ret_json)
        })
    }
    let streamStatus = {};
    fetch('/url-status',{
            method: "GET",
        }).then(res => res.json())
    .then(urlStatus => {
        for (const item of items) {
            streamStatus[item] = 'green';
            for (const url of $template[item].url) {
                if (urlStatus[url] === 1) {
                    streamStatus[item] = 'red';
                }
                if (urlStatus[url] === 2) {
                    streamStatus[item] = 'yellow';
                }
            }
        }
    })
</script>
<div class="flex flex-col w-72 h-screen px-4 pt-8 bg-white border-r overflow-auto"
     transition:fly={{delay: 400, x: -100}}>
    <div class="flex items-center px-3 -mx-2">
        <img class="object-cover rounded-full h-9 w-9" src="{face}" alt="avatar"/>
        <div  class="tooltip">
            <h4 class="mx-2 font-medium text-gray-800 dark:text-gray-200 hover:underline truncate">biliup</h4>
        </div>
        <Modal>
            <a slot="open-modal" class="flex cursor-pointer tooltip items-center" data-tip="设置" on:click={loadSettings} >
                <svg class="h-5 w-5" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor">
                    <path fill-rule="evenodd" d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z" clip-rule="evenodd" />
                </svg>
            </a>

            <div slot="box" let:componentId>
                <div class="space-y-2.5">
                    <h4>单视频并发数：{limit}</h4>
                    <input type="range" max="128" min="1" bind:value={limit} class="range  range-xs">
                    <!--                    <button class="btn btn-outline">线路: AUTO</button>-->
                    <h4>上传线路选择：</h4>
                    <div class="btn-group">
                        {#each lines as l}
                            <input type="radio" bind:group={line} value="{l}" data-title="{l}"  class="btn btn-outline">
                        {/each}
                    </div>
                    {#if !is_toml}
                        <div class="flex flex-col">
                            <label class="label">
                                <span class="text-sm font-bold text-gray-500 tracking-wide">SESSDATA</span>
                            </label>
                            <input bind:value={User.SESSDATA}
                                   class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                                   placeholder="SESSDATA">
                        </div>
                        <div class="flex flex-col">
                            <label class="label">
                                <span class="text-sm font-bold text-gray-500 tracking-wide">DedeUserID__ckMd5</span>
                            </label>
                            <input bind:value={User.DedeUserID__ckMd5}
                                   class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                                   placeholder="DedeUserID__ckMd5">
                        </div>
                        <div class="flex flex-col">
                            <label class="label">
                                <span class="text-sm font-bold text-gray-500 tracking-wide">bili_jct</span>
                            </label>
                            <input bind:value={User.bili_jct}
                                   class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                                   placeholder="bili_jct">
                        </div>
                        <div class="flex flex-col">
                            <label class="label">
                                <span class="text-sm font-bold text-gray-500 tracking-wide">access_token</span>
                            </label>
                            <input bind:value={User.access_token}
                                   class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                                   placeholder="access_token">
                        </div>
                        <div class="flex flex-col">
                            <label class="label">
                                <span class="text-sm font-bold text-gray-500 tracking-wide">DedeUserID</span>
                            </label>
                            <input bind:value={User.DedeUserID}
                                   class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                                   placeholder="DedeUserID">
                        </div>
                    {/if}
                </div>

                <div class="modal-action">
                    <label for="{componentId}" on:click={saveSettings} class="btn btn-accent">Save</label>
                    <label for="{componentId}" class="btn">Close</label>
                </div>
            </div>

        </Modal>

    </div>


    <div class="flex flex-col justify-between flex-1 mt-6">
        <nav class="">
            {#each items as item(item)}

                <a animate:flip="{{duration: 300}}" class:selected="{$currentTemplate.current === item}"
                   on:click="{() => select(item)}">
                    {#if streamStatus[item]}
                        <span class="flex absolute h-1.5 w-1.5 top-0 right-0 flex">
                          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-{streamStatus[item]}-400 opacity-75"></span>
                          <span class="relative inline-flex rounded-full h-1.5 w-1.5 bg-{streamStatus[item]}-500"></span>
                        </span>
                    {/if}
                    <svg class="flex-none w-5 h-5" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M19 11H5M19 11C20.1046 11 21 11.8954 21 13V19C21 20.1046 20.1046 21 19 21H5C3.89543 21 3 20.1046 3 19V13C3 11.8954 3.89543 11 5 11M19 11V9C19 7.89543 18.1046 7 17 7M5 11V9C5 7.89543 5.89543 7 7 7M7 7V5C7 3.89543 7.89543 3 9 3H15C16.1046 3 17 3.89543 17 5V7M7 7H17"
                              stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                    {#if $currentTemplate.current !== item}
                        <div out:send={{key: item}}></div>
                    {/if}
                    <span class="ml-4 font-medium truncate">{item}</span>
                </a>

            {/each}
        </nav>

        <div class="sticky bottom-0 bg-base-100">
            <button class="mt-2.5 mb-5 py-2 px-4 flex justify-center items-center bg-green-500 hover:bg-green-700 focus:ring-green-500 focus:ring-offset-green-200 text-white w-full transition ease-in duration-200 text-center text-base font-semibold shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2  rounded-full"
                    on:click={add}
                    type="button">
                <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                     xmlns="http://www.w3.org/2000/svg">
                    <path d="M12 4v16m8-8H4" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"/>
                </svg>
            </button>
        </div>
    </div>
</div>

<style>
    .selected {
        @apply text-gray-700 bg-gray-200;
    }

    nav > a {
        @apply flex cursor-pointer items-center px-3 py-2 mt-1 text-gray-600 transition-colors duration-200 transform rounded-md hover:bg-gray-200 hover:text-gray-700;
    }

    .status {
        @apply bg-yellow-400 bg-yellow-500 bg-green-400 bg-green-500 bg-red-400 bg-red-500;
    }

</style>
