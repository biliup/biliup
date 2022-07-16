<script lang="ts">
    import {currentTemplate, receive, template} from './store';
    import {archivePre, createPop, partition} from "./common";
    import {flip} from 'svelte/animate';


    export let selected;
    export let selectedTemplate;
    let oldSelected = selected;
    let uploaders = ["bili_web", "Noop"];
    // let title: string = ;
    let nocopyright: boolean;
    $ : nocopyright = selectedTemplate?.copyright === 2;
    let noReprint = true;

    function handleClick(e) {
        selectedTemplate.copyright = e.target.checked ? 2 : 1;
    }

    let edit = false;

    function update(e) {
        if (!e) {
            console.log(oldSelected);
            delete $template[oldSelected];
            oldSelected = selected
            selectedTemplate.changed = true;
            $template[selected] = selectedTemplate;
            console.log($template);
            let data = {
                "streamers": $template,
            }
            fetch('/api/setconfig', {
                method: 'POST',
                body: JSON.stringify(data),
                headers: {
                    'Content-Type': 'application/json'
                }
            })
        }
        edit = e;
    }

    async function del() {
        let len = Object.keys($template).length;
        const keys = Object.keys($template);
        const index = keys.indexOf(selected);
        if (len==1){
            createPop("已经是最后一个模板无法删除");
            return;
        }
        delete $template[selected];
        if (len > 1) {
            selected = keys[index + 1] || keys[index - 1];
            $currentTemplate.selectedTemplate = $template[selected];
            $currentTemplate.current = selected;
        } else {
            selected = '';
            $currentTemplate.selectedTemplate = null;
            $currentTemplate.current = '';
        }
        $template = $template;
        let data = {
            "streamers": $template
        }
        console.log($template);
        try {
            await fetch('/api/setconfig', {
                method: 'POST',
                body: JSON.stringify(data),
                headers: {
                    'Content-Type': 'application/json'
                }
            })
            createPop('移除成功', 2000, 'Success');
        } catch (e) {
            console.log(e);
            createPop(e, 5000);
        }

    }

    // console.log()
    let tags = selectedTemplate?.tags ? selectedTemplate?.tags : [];
    let urls = selectedTemplate?.url ? selectedTemplate?.url : [];
    // $: tags = selectedTemplate?.tag.split(',');

    let parent = '请选择';
    let children = '分区';
    let current;
    let currentChildren;
    $: {
        if ($partition) {
            // tags.flatMap()
            // $partition.flatMap()
            let changed = false;
            for (const partitionElement of $partition) {
                for (const child of partitionElement.children) {
                    if (child.id === selectedTemplate.tid) {
                        parent = partitionElement.name;
                        children = child.name;
                        current = partitionElement.id;
                        currentChildren = child.id;
                        changed = true;
                    }
                }
                // typeList = typeList.concat(partitionElement.children);
                // console.log(partitionElement.children);
            }
            if (!changed) {
                parent = '请选择';
                children = '分区';
                current = null;
                currentChildren = null;
            }
            // console.log(typeList);
        }
    }

    let tempTag;
    let tempUrl;


    function handleKeypress() {
        console.log(tags);
        if (tags.includes(tempTag)) {
            createPop("已有相同标签");
            tempTag = null;
            return;
        }
        if(tags.length > 12) {
            createPop("标签数量超过12个，无法添加");
            tempTag = null;
            return;
        }
        fetch('/api/check_tag?'+new URLSearchParams({
            tag:tempTag
        }), {
            method: 'get',
        }).then(res=>{
            if(!res.ok){
                createPop("标签违禁")
                tempTag = null;
            }else{
                tags = [...tags, tempTag];
                selectedTemplate.tags = tags;
                tempTag = null;
            }
        })


        // return false;
    }

    function urlhandleKeypress() {
        console.log(urls);
        if (urls.includes(tempUrl)) {
            createPop("已有相同链接");
            tempUrl = null;
            return;
        }
        urls = [...urls, tempUrl];
        selectedTemplate.url = urls;
        tempUrl = null;
        return false;
    }

    function removeTag(tag) {
        tags = tags.filter(t => t !== tag);
        selectedTemplate.tags = tags;
        console.log(tag);
    }

    function removeUrl(url) {
        urls = urls.filter(u => u !== url);
        selectedTemplate.url = urls;
        console.log(url);
    }


    function callback(detailTid, detailParent, detailChildren) {
        selectedTemplate.tid = detailTid;
        parent = detailParent;
        children = detailChildren;
    }

    let dtime;
    let isDtime = false;
    let date;
    let time;

    // a reference to the component, used to call FilePond methods

</script>
<div>
    <div class="shadow-md md:max-w-xl sm:max-w-sm lg:max-w-2xl w-screen px-10 pt-3 pb-10 mt-2 mb-2 bg-white rounded-xl"
         in:receive={{key: selected}}>
        <div class="space-y-3">
            <div class="flex flex-row-reverse">
                <button class="ml-2 py-2 px-2 flex justify-center items-center bg-red-600 hover:bg-red-700 focus:ring-red-500 focus:ring-offset-red-200 text-white transition ease-in duration-200 text-center text-base font-semibold shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2  w-8 h-8 rounded-lg "
                        on:click|preventDefault={del}
                        type="button">
                    <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                         xmlns="http://www.w3.org/2000/svg">
                        <path d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                              stroke-linecap="round" stroke-linejoin="round"
                              stroke-width="2"/>
                    </svg>
                </button>
                <!--                <button class="py-2 px-2 flex justify-center items-center bg-blue-600 hover:bg-blue-700 focus:ring-blue-500 focus:ring-offset-blue-200 text-white transition ease-in duration-200 text-center text-base font-semibold shadow-md focus:outline-none focus:ring-2 focus:ring-offset-2  w-8 h-8 rounded-lg " on:click|preventDefault={save}-->
                <!--                        type="button">-->
                <!--                    <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"-->
                <!--                         xmlns="http://www.w3.org/2000/svg">-->
                <!--                        <path d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" stroke-linecap="round" stroke-linejoin="round"-->
                <!--                              stroke-width="2"/>-->
                <!--                    </svg>-->
                <!--                </button>-->
            </div>
            <div class="flex flex-col">
                <label class="text-sm font-bold text-gray-500 tracking-wide mb-2">
                    {#if (edit)}
                        <input on:focusout={()=> update(false)} bind:value={selected}
                               class="w-full p-1 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                               placeholder="模板名称">
                    {:else}
                        <div class="p-1">
                            {selected}
                            <svg on:click={()=> update(true)}
                                 xmlns="http://www.w3.org/2000/svg"
                                 class="cursor-pointer inline h-5 w-5 hover:text-blue-700" fill="none"
                                 viewBox="0 0 24 24" stroke="currentColor">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                      d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/>
                            </svg>
                            <a>修改模板名称</a>
                        </div>
                    {/if}
                </label>
                <input bind:value={selectedTemplate.title}
                       class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"
                       placeholder="标题">

            </div>
            <!--            <div class="flex flex-col">-->
            <!--                <label class="label">-->
            <!--                    <span class="text-sm font-bold text-gray-500 tracking-wide">直播间链接</span>-->
            <!--                </label>-->
            <!--                <input bind:value={selectedTemplate.url}-->
            <!--                       class="text-base p-2 border border-gray-300 rounded-lg focus:outline-none focus:border-indigo-500"-->
            <!--                       placeholder="www.douyu.com/3484">-->
            <!--            </div>-->
            <div class="flex flex-wrap rounded-lg border border-gray-300 focus:outline-none focus:ring-2 focus:ring-purple-600 focus:border-transparent">
                {#each urls as url(url)}
                    <span animate:flip="{{duration: 300}}"
                          class="flex  ml-1 my-1.5 px-3 py-0.5 text-base rounded-full text-white  bg-indigo-500 ">
                        {url}
                        <button on:click={(e)=>{removeUrl(url)}} class="bg-transparent hover">
                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor"
                                 class="ml-2" viewBox="0 0 1792 1792">
                                <path d="M1490 1322q0 40-28 68l-136 136q-28 28-68 28t-68-28l-294-294-294 294q-28 28-68 28t-68-28l-136-136q-28-28-28-68t28-68l294-294-294-294q-28-28-28-68t28-68l136-136q28-28 68-28t68 28l294 294 294-294q28-28 68-28t68 28l136 136q28 28 28 68t-28 68l-294 294 294 294q28 28 28 68z">
                                </path>
                            </svg>
                        </button>
                    </span>
                {/each}

                <input bind:value={tempUrl}
                       class="outline-none rounded-lg flex-1 appearance-none w-full py-2 px-4 bg-white text-gray-700 placeholder-gray-400 shadow-sm text-base "
                       on:keypress={e=>e.key==='Enter' && urlhandleKeypress()}
                       placeholder="录制链接, 回车输入"
                       type="text"/>
            </div>


            <div class="mb-3 flex justify-between items-center">
                <!--                <div>-->
                <!--                    <div class="relative inline-block w-10 mr-2 align-middle select-none">-->
                <!--                        bind:checked={nocopyright}-->
                <input checked={nocopyright} class="toggle" id="Orange"
                       name="toggle" on:change={(event) => handleClick(event)}
                       type="checkbox"/>
                <!--                        <label class="block overflow-hidden h-6 rounded-full bg-gray-100 cursor-pointer" for="Orange">-->
                <!--                        </label>-->
                <!--                    </div>-->
                <span class="mx-2 w-auto text-sm text-gray-500 tracking-wide">
                            是否转载
                    </span>
                <!--                </div>-->
                <div class="pl-4 invisible flex-grow" class:copyright={nocopyright}>
                    <input bind:value={selectedTemplate.source} class="input input-bordered w-full" id="rounded-email"
                           placeholder="转载来源"
                           type="text"/>
                </div>
            </div>
            {#if !nocopyright}
                <div class="form-control">
                    <label class="label cursor-pointer">
                        <span class="label-text">自制声明：未经作者授权 禁止转载</span>
                        <input type="checkbox" bind:checked="{noReprint}" class="checkbox">
                    </label>
                </div>
            {/if}
            <div class="flex">
                <div class="flex w-52" use:archivePre={{callback, current, currentChildren}}>
                    <button class="border border-gray-300 relative w-full bg-white rounded-md pl-3 pr-10 py-3 text-left cursor-default focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"
                            type="button">
                    <span class="flex items-center">
                        <span class="ml-1 block truncate">
                            {parent} → {children}
                        </span>
                    </span>
                        <span class="ml-3 absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">
                        <svg aria-hidden="true" class="h-5 w-5 text-gray-400" fill="currentColor"
                             viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
                            <path clip-rule="evenodd"
                                  d="M10 3a1 1 0 01.707.293l3 3a1 1 0 01-1.414 1.414L10 5.414 7.707 7.707a1 1 0 01-1.414-1.414l3-3A1 1 0 0110 3zm-3.707 9.293a1 1 0 011.414 0L10 14.586l2.293-2.293a1 1 0 011.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z"
                                  fill-rule="evenodd">
                            </path>
                        </svg>
                    </span>
                    </button>
                    <!--                <input bind:this={archivePre} bind:value={tid} type="text" class=" rounded-lg border-transparent flex-1 appearance-none border border-gray-300 w-full py-2 px-4 bg-white text-gray-700 placeholder-gray-400 shadow-sm text-base focus:outline-none focus:ring-2 focus:ring-purple-600 focus:border-transparent" placeholder="分区"/>-->
                </div>
<!--                <div class="flex w-48">-->
<!--                    <select class="border border-gray-300 relative w-full bg-white rounded-md pl-3 pr-10 py-3 text-left cursor-default focus:outline-none focus:ring-1 focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm"-->
<!--                            type="button" bind:value={selectedTemplate.uploader}>-->

<!--                        {#each uploaders as uploader}-->
<!--                            <option class="text-xl" value={uploader}>-->
<!--                                {uploader}-->
<!--                            </option>-->
<!--                        {/each}-->

<!--                        <span class="ml-3 absolute inset-y-0 right-0 flex items-center pr-2 pointer-events-none">-->
<!--                        <svg aria-hidden="true" class="h-5 w-5 text-gray-400" fill="currentColor"-->
<!--                             viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">-->
<!--                            <path clip-rule="evenodd"-->
<!--                                  d="M10 3a1 1 0 01.707.293l3 3a1 1 0 01-1.414 1.414L10 5.414 7.707 7.707a1 1 0 01-1.414-1.414l3-3A1 1 0 0110 3zm-3.707 9.293a1 1 0 011.414 0L10 14.586l2.293-2.293a1 1 0 011.414 1.414l-3 3a1 1 0 01-1.414 0l-3-3a1 1 0 010-1.414z"-->
<!--                                  fill-rule="evenodd">-->
<!--                            </path>-->
<!--                        </svg>-->
<!--                    </span>-->
<!--                    </select>-->
<!--                </div>-->

            </div>

            <div class="flex flex-wrap rounded-lg border border-gray-300 focus:outline-none focus:ring-2 focus:ring-purple-600 focus:border-transparent">
                {#each tags as tag(tag)}
                    <span animate:flip="{{duration: 300}}"
                          class="flex  ml-1 my-1.5 px-3 py-0.5 text-base rounded-full text-white  bg-indigo-500 ">
                        {tag}
                        <button on:click={(e)=>{removeTag(tag)}} class="bg-transparent hover">
                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" fill="currentColor"
                                 class="ml-2" viewBox="0 0 1792 1792">
                                <path d="M1490 1322q0 40-28 68l-136 136q-28 28-68 28t-68-28l-294-294-294 294q-28 28-68 28t-68-28l-136-136q-28-28-28-68t28-68l294-294-294-294q-28-28-28-68t28-68l136-136q28-28 68-28t68 28l294 294 294-294q28-28 68-28t68 28l136 136q28 28 28 68t-28 68l-294 294 294 294q28 28 28 68z">
                                </path>
                            </svg>
                        </button>
                    </span>
                {/each}

                <input bind:value={tempTag}
                       class="outline-none rounded-lg flex-1 appearance-none  w-full py-2 px-4 bg-white text-gray-700 placeholder-gray-400 shadow-sm text-base "
                       on:keypress={e=>e.key==='Enter' && handleKeypress()}
                       placeholder="标签"
                       type="text"/>
            </div>
            <div class="text-gray-700">
                <label class="label">
                    <span class="text-sm font-bold text-gray-500 tracking-wide">简介</span>
                </label>
                <textarea bind:value={selectedTemplate.description}
                          class="textarea textarea-bordered w-full"
                          cols="40" placeholder="简介补充: ..." rows="4"></textarea>
            </div>
            <div class="text-gray-700">
                <label class="label">
                    <span class="text-sm font-bold text-gray-500 tracking-wide">粉丝动态</span>
                </label>
                <textarea bind:value={selectedTemplate.dynamic}
                          class="textarea textarea-bordered w-full"
                          cols="40" placeholder="动态描述" rows="1"></textarea>
            </div>
            <div class="flex items-center">
                <input type="checkbox" class="toggle my-2" bind:checked="{isDtime}">
                <span class="ml-2 text-sm font-bold text-gray-500 tracking-wide">开启定时发布</span>
                {#if (isDtime)}
                    <input class="mx-3 border rounded-lg border-gray-300 py-1 px-2" type="date" bind:value={date}/>
                    <input class="mx-3 border rounded-lg border-gray-300 py-1 px-2" type="time" bind:value={time}/>
                {/if}
            </div>
            <!--{#if (autoSubmit)}-->
            <!--    <div class="flex justify-center items-center">-->
            <!--        <button type="button" class="inline-flex items-center px-4 py-2 font-semibold leading-6 text-sm shadow rounded-md text-white bg-indigo-500 hover:bg-indigo-400 transition ease-in-out duration-150 cursor-not-allowed" disabled>-->
            <!--            <svg class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">-->
            <!--                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>-->
            <!--                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>-->
            <!--            </svg>-->
            <!--            等待视频上传完后会自动提交...-->
            <!--        </button>-->
            <!--        <a class="cursor-pointer" on:click={cancelSubmit}>-->
            <!--            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-red-400 hover:stroke-rose-500 transition ease-in-out duration-150 ml-2.5 h-7 w-7" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">-->
            <!--                <path stroke-linecap="round" stroke-linejoin="round" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />-->
            <!--            </svg>-->
            <!--        </a>-->
            <!--    </div>-->
            <!--{:else}-->
            <!--    <button class="p-2 my-5 w-full flex justify-center bg-blue-500 text-gray-100 rounded-full tracking-wide-->
            <!--              font-semibold  focus:outline-none focus:shadow-outline hover:bg-blue-600 shadow-lg cursor-pointer transition ease-in duration-300" on:click|preventDefault={submit} type="submit">-->
            <!--        提交配置-->
            <!--    </button>-->
            <!--{/if}-->
        </div>
    </div>
</div>

<style>
    .copyright {
        @apply visible;
    }
</style>
