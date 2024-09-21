import React, {useEffect, useState} from "react";
import {fetcher, sendRequest} from "@/app/lib/api-streamer";
import {QRCodeSVG} from "qrcode.react";
import {Notification, Spin, Typography} from "@douyinfe/semi-ui";

type QrcodeProps = {
    onSuccess: (e: string) => void
}

const Qrcode: React.FC<QrcodeProps> = ({onSuccess}) => {
    const [url, setUrl] = useState('');
    useEffect(() => {
        // Create an instance.
        const controller = new AbortController();
        const signal = controller.signal;
        // Register a listenr.
        signal.addEventListener('abort', () => {
            console.log('aborted!');
        });
        (async () => {
            let qrData = await fetcher('/v1/get_qrcode', undefined);
            setUrl(qrData['data']['url']);
            console.log(qrData['data']['url']);
            let res = await fetch(`${process.env.NEXT_PUBLIC_API_SERVER ?? ''}/v1/login_by_qrcode`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(qrData),
                signal: signal,
            });
            if(!(res.status >=200 && res.status < 300)) {
                throw new Error(await res.text());
            }
            const data =  await res.json();
            if (!res.ok) {
                throw new Error(data.message);
            }
            onSuccess(data['filename']);
        })().catch((e) => {
            console.log(e);
            Notification.error({
                title: 'QRcode',
                content: <Typography.Paragraph style={{maxWidth: 450}}>{e.message}</Typography.Paragraph>,
                style: {width: 'min-content'}
            });
        });

        return () => {
            controller.abort('qrcode exit');
        };
    }, [onSuccess])
    if (url === '') {
        return <Spin/>
    }
    return <div style={{
        marginTop: 30,
        marginLeft: 'auto',
        marginRight: 'auto',
        width: 'max-content'
    }}>
        <QRCodeSVG value={url}/>
    </div>;
}

export default Qrcode;