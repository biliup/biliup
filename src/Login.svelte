<script lang="ts">
    import {isLogin} from './store.js';
    import {fade, scale} from 'svelte/transition';
    import {createPop, notifyHistory} from "./common";
    import QrCode from 'svelte-qrcode';

    let rememberMe: boolean = false;
    let username;
    let password;
    let loginMethod = "password";
    let countryCode = 86;
    let telephone: number = null;
    let verificationCode = null;
    let qrcode;
    let ret;
    let throttle = false;
    let maxtime = 60;


    function login_by_sms() {
        createPop("Not Support", 5000);
        return
        fetch('/api/login_by_sms', {
            method: 'POST'
        })
            .then((res) => {
                fetch('login_by_cookie')
                    .then((res) => {
                        isLogin.set(true);
                        console.log(`Message: ${res}`)
                    }).catch((e) => createPop(e, 5000))
            }).catch((e) => createPop(e, 5000))
    }

    function sendSms() {
        createPop("Not Support", 5000);
        return
        throttle = true;

        function CountDown() {
            if (maxtime >= 0) {
                --maxtime;
            } else {
                throttle = false;
                clearInterval(timer);
                maxtime = 60;
            }
        }

        let timer = setInterval(CountDown, 1000);
        fetch('/api/send_sms', {
            method: 'POST',
            body: JSON.stringify({
                bodycountrycode: countryCode,
                phone: telephone
            })
        })
            .then((res) => ret = res).catch((e) => createPop(e, 5000));
    }

    function loginByQrcode() {
        loginMethod = "qrcode";
        fetch('/api/get_qrcode')
            .then(ret => ret.json()).then((res) => {
            qrcode = res["data"]["url"];
            fetch('/api/login_by_qrcode', {
                method: 'POST',
                body: JSON.stringify(res),
            }).then((res) => {
                if (res.ok) {
                    fetch('/api/login_by_cookie')
                        .then((res) => {
                            isLogin.set(true)
                            console.log(`Message: ${res}`)
                        }).catch((e) => createPop(e, 5000));
                } else {
                    createPop(res.body, 5000);
                }

            }).catch((e) => {
                createPop(e, 5000);
                console.log(e);
            })
        }).catch((e) => {
            createPop(e, 5000);
            console.log(e);
        }
    )
    }

    async function browser() {
        let ret = await fetch('/api/get_qrcode');
        let res = await ret.json();
        await open(res["data"]["url"],'','noreferrer,popup');
        await fetch('/api/login_by_qrcode', {
            method: 'POST',
            body: JSON.stringify(res),
        })
        await fetch('/api/login_by_cookie')
        .then((res) => {
            if (res.ok) {
                isLogin.set(true);
                console.log(`Message: ${res}`);
            } else {
                $notifyHistory = [...$notifyHistory, {
                    type: 'Error',
                    msg: '校验Cookies失败',
                    date: new Date(),
                    duration: 3000
                }];
                isLogin.set(false);
            }
        })
        console.log(`Message: ${res}`);
    }

    function login() {
        createPop("Not Support", 5000);
        return

        console.log(rememberMe);
        invoke('login', {username: username, password: password, rememberMe: rememberMe})
            .then((res) => {
                isLogin.set(true);
                console.log(`Message: ${res}`)
            }).catch((e) => {
            createPop(e, 5000);
            console.log(e);
            // e = JSON.parse(e);
            // {"code":0,"data":{"cookie_info":null,"message":
            // "本次登录环境存在风险, 需使用手机号进行验证或绑定",
            // "sso":null,"status":2,
            // "token_info":null,
            // "url":"https://passport.bilibili.com/account/mobile/security/managephone/phone/verify?tmp_token=&requestId=&source=risk"},
            // "message":"0","ttl":1}
            // const webview = new WebviewWindow('theUniqueLabel', {
            //     url: e.data.url
            // })
            // createPop(JSON.stringify(e), 5000);
        })
    }
</script>
<div class="abs min-h-screen flex flex-col sm:justify-center items-center bg-white " transition:fade>
    <div class="relative sm:max-w-sm w-full" transition:scale>
        <div class="card bg-blue-400 shadow-lg  w-full h-full rounded-3xl absolute transform -rotate-6"></div>
        <div class="card bg-red-400 shadow-lg  w-full h-full rounded-3xl absolute transform rotate-6"></div>
        <div class="relative w-full rounded-3xl px-10 py-5 bg-zinc-50 shadow-md">

            {#if loginMethod === "password"}
                <div class="indicator w-full -mx-10">
                    <button class="indicator-item link flex text-sm" on:click={browser}>浏览器登录
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24"
                             stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                  d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                        </svg>
                    </button>
                </div>
                <form>
                    <label class="block text-sm text-gray-800">用户名</label>
                    <input bind:value={username}
                           class="block w-full px-4 py-2 mt-2 text-gray-700 bg-white border rounded-md focus:border-blue-500 focus:outline-none focus:ring"
                           type="text">

                    <div class="mt-4">
                        <div class="flex items-center justify-between">
                            <label class="block text-sm text-gray-800" for="password">密码</label>
                        </div>

                        <input bind:value={password}
                               class="block w-full px-4 py-2 mt-2 text-gray-700 bg-white border rounded-md focus:border-blue-500 focus:outline-none focus:ring"
                               type="password">
                    </div>
                    <label class="flex items-center mt-4">
                        <input bind:checked={rememberMe} class="form-checkbox" type="checkbox"/>
                        <span class="block ml-2 text-xs font-medium text-gray-700 cursor-pointer">Remember me</span>
                    </label>
                    <div class="mt-6">
                        <button class="w-full px-4 py-2 tracking-wide text-white transition-colors duration-200 transform bg-gray-700 rounded-md hover:bg-gray-600 focus:outline-none focus:bg-gray-600"
                                on:click|preventDefault={login}>
                            登录
                        </button>
                    </div>
                </form>
            {:else if loginMethod === "qrcode"}
                <div class="flex items-center justify-center">
                    <QrCode background="#fafafa" value={qrcode}/>
                </div>
            {:else if loginMethod === "sms"}
                <div class="form-control">
                    <label class="label">
                        <span class="label-text">国家代码+手机号</span>
                    </label>
                    <div class="flex">
                        <input type="text" placeholder="国家代码" bind:value={countryCode}
                               class="input input-bordered w-[5.75rem]">
                        <input type="text" placeholder="phone numbers" bind:value={telephone} class="input ml-2 w-full">
                    </div>
                    <label class="label">
                        <span class="label-text">验证码</span>
                    </label>
                    <div class="relative">
                        <input type="text" placeholder="verification code" bind:value={verificationCode}
                               class="w-full pr-16 input input-primary">
                        <button on:click={sendSms} class:loading={throttle} disabled={throttle}
                                class="absolute top-0 right-0 rounded-l-none btn btn-primary">
                            {#if !throttle}
                                发送验证码
                            {:else}
                                已发送（
                                <span class="countdown">
                                  <span style="--value:{maxtime};"></span>
                                </span>）
                            {/if}
                        </button>
                    </div>
                </div>
                <button class="btn btn-block mt-2.5" on:click={login_by_sms}>登录</button>
            {/if}
            <div class="flex items-center justify-between mt-4">
                <span class="w-1/5 border-b dark:border-gray-600 lg:w-1/5"></span>

                <a on:click={() => loginMethod='password'} href="#"
                   class="text-xs text-center text-gray-500 uppercase dark:text-gray-400 hover:underline">若登录失败，尝试其他登录方式</a>

                <span class="w-1/5 border-b dark:border-gray-400 lg:w-1/5"></span>
            </div>

            <div class="flex items-center mt-6 -mx-2">
                <button type="button" on:click={() => loginMethod = "sms"}
                        class="flex items-center justify-center w-full px-6 py-2 mx-2 text-sm font-medium text-white transition-colors duration-200 transform bg-blue-500 rounded-md hover:bg-blue-400 focus:bg-blue-400 focus:outline-none">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24"
                         stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                              d="M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z"/>
                    </svg>
                    <span class="hidden mx-2 sm:inline">短信登录</span>
                </button>

                <a href="#" on:click={loginByQrcode}
                   class="flex items-center justify-center px-6 py-2 mx-2 w-full text-sm font-medium text-gray-500 transition-colors duration-200 transform bg-gray-300 rounded-md hover:bg-gray-200">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24"
                         stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                              d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V5a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1zm12 0h2a1 1 0 001-1V5a1 1 0 00-1-1h-2a1 1 0 00-1 1v2a1 1 0 001 1zM5 20h2a1 1 0 001-1v-2a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1z"/>
                    </svg>
                    <!--                    <svg class="w-5 h-5 fill-current" viewBox="0 0 24 24">-->
                    <!--                        <path-->
                    <!--                                d="M23.954 4.569c-.885.389-1.83.654-2.825.775 1.014-.611 1.794-1.574 2.163-2.723-.951.555-2.005.959-3.127 1.184-.896-.959-2.173-1.559-3.591-1.559-2.717 0-4.92 2.203-4.92 4.917 0 .39.045.765.127 1.124C7.691 8.094 4.066 6.13 1.64 3.161c-.427.722-.666 1.561-.666 2.475 0 1.71.87 3.213 2.188 4.096-.807-.026-1.566-.248-2.228-.616v.061c0 2.385 1.693 4.374 3.946 4.827-.413.111-.849.171-1.296.171-.314 0-.615-.03-.916-.086.631 1.953 2.445 3.377 4.604 3.417-1.68 1.319-3.809 2.105-6.102 2.105-.39 0-.779-.023-1.17-.067 2.189 1.394 4.768 2.209 7.557 2.209 9.054 0 13.999-7.496 13.999-13.986 0-.209 0-.42-.015-.63.961-.689 1.8-1.56 2.46-2.548l-.047-.02z">-->
                    <!--                        </path>-->
                    <!--                    </svg>-->
                    <span class="hidden mx-2 sm:inline">扫码登录</span>
                </a>
            </div>
        </div>
    </div>
</div>

<style>
    .abs {
        /*overflow-y: overlay;*/
        margin-right: calc(100% - 100vw);
    }
</style>