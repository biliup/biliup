<script>
    import {attach, fileselect} from './store.ts';
    import {draw} from 'svelte/transition';
    import {getContext, onDestroy} from "svelte";


    let draggedOver = false;
    // $: draggedOver = getContext("hover");
    console.log("1", getContext("hover"));
    const unsubscribe = getContext("hover")?.subscribe(value => {
        draggedOver = value;
    });

    onDestroy(unsubscribe);
    // use to check if a file is being dragged
    const hasFiles = ({dataTransfer: {types = []}}) =>
        types.indexOf("Files") > -1;

    // use to drag dragenter and dragleave events.
    // this is to know if the outermost parent is dragged over
    // without issues due to drag events on its children
    let counter = 0;

    // reset counter and append file to gallery when file is dropped
    function dropHandler(event) {
        console.log(event)
        attach(event.dataTransfer.files);
        draggedOver = false;
        counter = 0;
    }

    // only react to actual files being dragged
    function dragEnterHandler(e) {
        if (!hasFiles(e)) {
            return;
        }
        ++counter;
        draggedOver = true;
    }

    function dragLeaveHandler(e) {
        if (1 > --counter) {
            draggedOver = false;
        }
    }

    function dragOverHandler(e) {
        if (hasFiles(e)) {
            e.preventDefault();
        }
    }
</script>


<div class="bg-white rounded-lg relative flex items-center justify-center w-full"
     on:dragenter|preventDefault="{dragEnterHandler}" on:dragleave="{dragLeaveHandler}" on:dragover="{dragOverHandler}"
     on:drop|preventDefault="{dropHandler}">
    {#if (draggedOver)}
        <div id="overlay"
             class="draggedover w-full h-full absolute top-0 left-0 pointer-events-none z-50 flex flex-col items-center justify-center rounded-md">
            <i>
                <svg xmlns="http://www.w3.org/2000/svg" class="w-10 h-10 text-blue-400 group-hover:text-blue-600"
                     fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path transition:draw stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                          d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                </svg>
            </i>
            <p class="text-lg text-blue-700">Drop files to upload</p>
        </div>
    {/if}

    <label class="flex flex-col items-center justify-center cursor-pointer rounded-lg border-2 border-dashed w-full h-full p-5 group text-center"
           on:click={fileselect}>

        <img alt="freepik image" class="max-h-48 w-3/5 object-center" src="image-upload-concept-landing-page.png">

        <p class="mt-4 text-gray-500 "><span class="text-sm">拖拽</span> 视频到此处 <br/>或者从你的电脑中<a
                class="text-blue-600 hover:underline">选择</a></p>
        <!--        <input on:change={(event)=>attach(event.target.files)} on:change={(event)=> event.target.value=null}-->
        <!--               type="file" class="hidden" multiple accept=".mp4,.flv,.avi,.wmv,.mov,.webm,.mpeg4,.ts,.mpg,.rm,.rmvb,.mkv,.m4v">-->
    </label>
</div>

<style>

    #overlay.draggedover {
        background-color: rgba(255, 255, 255, 0.7);
    }

    #overlay.draggedover p, #overlay.draggedover i {
        opacity: 1;
    }

    .group:hover .group-hover\:text-blue-800 {
        color: #2b6cb0;
    }
</style>
