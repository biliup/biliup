<script lang="ts">
    import Sidebar from './Sidebar.svelte';
    import Upload from './Upload.svelte';
    import {currentTemplate, template} from "./store";
    import {createPop} from "./common";


    let map;
    fetch('http://localhost:19159/api/getconfig',{
        method: 'GET',
    }).then((res=>res.json())).then(data=>{
        map = data;

        console.log(map);
        $template = map;
        console.log($template);
        let key = Object.keys($template)[0];
        $currentTemplate.current = key;
        $currentTemplate.selectedTemplate = $template[key];
        console.log(data);
    }).catch((e) => {
            createPop(e);
            console.log(e);
        }
    )
    function submit(){
        let data={
            "streamers":$template
        }
        fetch('http://localhost:19159/api/setconfig',{
            method: 'POST',
            body: JSON.stringify(data),
            headers: {
                'Content-Type': 'application/json'
            }
        }).then((res=>res.json())).catch((e) => {
                createPop(e);
                console.log(e);
            }
        )
        fetch('http://localhost:19159/api/save').then(res =>{
            if (res.ok){
                createPop(`配置已保存`, 5000, 'Success');
            }
        }).catch((e) => {
                createPop(e);
                console.log(e);
            }
        )
    }
    let items = [];
    $: items = [...Object.keys($template)];
</script>

<div class="flex items-start">
    <Sidebar items="{items}"/>
    <div class="grid justify-center w-screen h-screen rhs overflow-y-auto overflow-x-hidden">
        <div class="grid items-center justify-around min-h-screen">
            <!--        <Upload selected={current}/>-->
            {#key $currentTemplate.current}
                <Upload selected={$currentTemplate.current} selectedTemplate="{$currentTemplate.selectedTemplate}"/>
            {/key}
            <button class="p-2 my-5 w-full flex justify-center bg-blue-500 text-gray-100 rounded-full tracking-wide
                          font-semibold  focus:outline-none focus:shadow-outline hover:bg-blue-600 shadow-lg cursor-pointer transition ease-in duration-300" on:click|preventDefault={submit} type="submit">
                提交录制配置
            </button>
        </div>

    </div>
</div>



