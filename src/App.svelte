<script lang="ts">
    import Home from './Home.svelte';
    import Modal from "./Modal.svelte";
    import {notifyHistory} from "./common";
</script>

<main class="bg-gray-100" on:dragenter|preventDefault on:dragleave|preventDefault on:dragover|preventDefault
      on:drop|preventDefault>
        <Home/>
    <div class="fixed space-y-2.5 top-0 right-0 w-1/2" id="alerts">
    </div>
        <ul class="menu bg-base-100 rounded-box fixed bottom-16 right-3 w-max drop-shadow">
            <li>
                <Modal>
                    <a slot="open-modal">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                        </svg>
                    </a>

                    <div slot="box" class="px-4 border-b rounded-t sm:px-6">
                        <div class="bg-white my-2 shadow sm:rounded-md">
                            <ul class="divide-y divide-gray-200">
                                {#each $notifyHistory as notify}
                                    <li>
                                        <div class="indicator w-full">
                                            <span class="!left-auto indicator-item indicator-top indicator-center">
                                                <label class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800">
                                                    {notify.date.toLocaleString()}
                                                    <!--{new Date().toLocaleString()}-->
                                                </label>
                                            </span>
                                            <div class="flex items-center px-2 pt-4 pb-2 hover:bg-gray-50 ">
                                                {#if notify.type === 'Error'}
                                                    <svg xmlns="http://www.w3.org/2000/svg" class="mr-2 stroke-warning flex-shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
                                                {:else }
                                                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="mr-2 stroke-info flex-shrink-0 w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                                                {/if}
                                                <p class="text-gray-700">
                                                    { notify.msg }
                                                </p>
                                            </div>
                                        </div>
                                    </li>
                                {/each}
                            </ul>
                        </div>
                        <p class="text-right text-sm text-slate-500">
                            通知历史
                        </p>
                    </div>
                </Modal>

            </li>
            <li>
                <Modal>
                    <a slot="open-modal">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                    </a>
                    <div slot="box" class="">
                        <h1 class="text-2xl font-bold text-center">
                            biliup-ui
                            <span class="badge badge-primary">v0.0.1</span>
                        </h1>
                        <p class="mt-3">
                            欢迎使用，如果操作有一些疑问，可以先查看
                            <a class="text-blue-600 after:content-['_↗'] ..." href="https://forgqi.github.io/biliup-app " target="_blank">文档</a>
                            里面有使用演示视频，下载地址（建议保持更新到最新版）与源码的地址。如果有什么建议可以在GitHub的issue中提出。
                            这个软件能帮到你的话，可以在GitHub上点个star，谢谢！
                    </div>
                </Modal>
            </li>
        </ul>
</main>

<style global lang="postcss">
    @tailwind base;
    @tailwind components;
    @tailwind utilities;
</style>
