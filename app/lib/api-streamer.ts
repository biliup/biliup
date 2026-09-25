// Fetcher implementation. // The extra argument will be passed via the `arg` property of the 2nd parameter.// In the example below, `arg` will be `'my_token'`
import { Toast } from '@douyinfe/semi-ui';
import { mutate } from 'swr';

export const API_BASE = process.env.NEXT_PUBLIC_API_SERVER ?? '';

/** 当前用户与权限点（见 use-me.ts）。放在这里是为了让统一的响应处理能刷新它，又不形成循环引用。 */
export const ME_KEY = '/v1/me';

/**
 * 重新拉取当前用户与权限点。收到 403、长连接意外断开时调用：
 * 会话已失效时 /v1/me 返回 401 → 跳登录页；角色被降级时页面按新的权限点收起。
 */
export function revalidateMe() {
	mutate(ME_KEY).catch(() => undefined);
}
export async function sendRequest<T>(url: string, { arg }: { arg: T }) {
	const res = await fetch(API_BASE + url, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(arg),
	});
	await handleResponse(res);
	return res.json();
}

export const fetcher = async (input: RequestInfo | URL, init?: RequestInit) => {
	const res = await fetch(API_BASE + input, init);
	await handleResponse(res);
	return res.json();
};

export const proxy = async (input: RequestInfo | URL, init?: RequestInit) => {
	const res = await fetch(API_BASE + input, init);
	await handleResponse(res);
	return res;
};

export async function requestDelete<T>(url: string, { arg }: { arg: T }) {
	const res = await fetch(`${API_BASE}${url}/${arg}`, { method: 'DELETE' });
	await handleResponse(res);
	return res;
}

export async function put<T>(url: string, { arg }: { arg: T }) {
	const res = await fetch(`${API_BASE}${url}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(arg),
	});
	await handleResponse(res);
	return res;
}

/** 统一的响应处理：401 跳登录页，403 提示并刷新权限点，其它失败抛出服务端信息。 */
export async function handleResponse(res: Response) {
	// 如果未登录，统一跳转
	if (res.status === 401) {
		// 可选：清理本地状态/缓存
		// localStorage.removeItem('token') 等

		// 跳转登录（带回跳）
		const returnTo = encodeURIComponent(window.location.pathname + window.location.search);
		window.location.href = `/login?next=${returnTo}`;
		// 抛错让 SWR 知道失败（别返回 json）
		throw new Error('Unauthorized');
	}

	// 已登录但没有这项权限：留在当前页，提示一次即可（同 id 的 Toast 不会叠加）。
	// 多半是权限点在别处被改了，顺手刷新，让界面按新的权限收起
	if (res.status === 403) {
		revalidateMe();
		const text = await res.text().catch(() => '');
		let message = text.trim();
		try {
			message = JSON.parse(text)?.message ?? message;
		} catch {
			// 纯文本错误信息，原样展示
		}
		message ||= '没有权限执行此操作';
		Toast.warning({ id: 'forbidden', content: message });
		throw new Error(message);
	}

	if (!res.ok) {
		// 尽量返回服务端错误信息
		const text = await res.text().catch(() => '');
		throw new Error(text || `HTTP ${res.status}`);
	}
	return res;
}

type Credit = {
	username: string;
	uid: number;
};

export interface StudioEntity {
	id: number;
	template_name: string;
	user_cookie: string;
	copyright: number;
	copyright_source: string;
	tid: number;
	tid_v2?: number | null;
	cover_path: string;
	title: string;
	description: string;
	dynamic: string;
	tags: string[];
	dtime: number;
	// interactive: number;
	mission_id?: number;
	dolby: number;
	hires: number;
	no_reprint: number;
	is_only_self: number;
	up_selection_reply: number;
	up_close_reply: number;
	up_close_danmu: number;
	charging_pay: number;
	credits: Credit[];
	uploader: string;
	extra_fields?: string;
}

/** 正在录制的直播间能否在页面内预览（复用正在录制的那一路流，见 GET /v1/streamers/{id}/live） */
export interface LivePreviewInfo {
	/** 当前下载器能否提供预览；为 true 时 format 仍可能是 null（刚开始拉流、容器未定） */
	available: boolean;
	/** 视频流的容器，与 /live 响应的 Content-Type 一致：flv / mpegts 走 mpegts.js，fmp4 直接 MediaSource */
	format: 'flv' | 'mpegts' | 'fmp4' | null;
	/** fmp4 时从 init segment 解出的 RFC 6381 编码串（如 avc1.64001f,mp4a.40.2），其它容器为 null */
	codecs: string | null;
	/** 不可预览的原因（ffmpeg / streamlink 子进程落盘、HEVC FLV 等） */
	reason: string | null;
	/** 这一路有没有实时弹幕（平台实现了弹幕客户端），有则 /v1/streamers/{id}/danmaku（SSE）可用 */
	danmaku: boolean;
	/** 浏览器能否直连 CDN 拉这一路（全局配置 preview_transport = direct 时用；不能则回落中转并显示原因） */
	direct: DirectCapability;
}

export interface DirectCapability {
	capable: boolean;
	/** 不能直连的原因；capable 为 true 时为 null */
	reason: string | null;
}

/** GET /v1/streamers/{id}/live-url：正在录制的那条流的 CDN 直链 */
export interface LiveUrlInfo {
	url: string;
	/** 浏览器该用哪个播放器：flv → mpegts.js，hls → hls.js（TS / fMP4 分片都行） */
	format: 'flv' | 'hls' | null;
	platform: string;
	/** 过期时间估计（Unix 秒），直链里没有可识别的过期参数时为 null */
	expires_at: number | null;
	direct: DirectCapability;
	/** 是后端 5 s 去抖窗口内复用的上一次结果，不是新取的 */
	cached: boolean;
}

/** 直播预览的取流方式（全局配置 preview_transport），空值视同 relay */
export type PreviewTransport = 'relay' | 'direct';

export interface LiveStreamerEntity {
	id: number;
	url: string;
	remark: string;
	filename: string;
	split_time?: number;
	split_size?: number;
	filename_prefix?: string;
	upload_id?: number;
	upload_streamers_id?: number | null;
	status?: string;
	upload_status?: string;
	/** 录制中最近一个滑动窗口的写盘速率（字节/秒）；未录制 / 尚无采样 / 下载器不支持时为 null */
	live_bytes_per_sec?: number | null;
	/** 录制中的直播间封面地址；图片请走 /v1/streamers/{id}/cover 代理 */
	live_cover_url?: string | null;
	/** 录制中的主播头像地址；图片请走 /v1/streamers/{id}/avatar 代理 */
	live_avatar_url?: string | null;
	/** 录制中的预览能力；未录制为 null */
	preview?: LivePreviewInfo | null;
	/** 正在录的场次（切片工作台）；未录制或第一个分段还没开写时为 null */
	session_id?: number | null;
	/** 正在录的这一场已经打了几个标记；session_id 为 null 时也为 null */
	marker_count?: number | null;
	statusTag?: React.ReactNode;
	format?: string;
    time_range?: string | Date[];
    excluded_keywords?: string[];
	preprocessor?: Record<'run', string>[];
	segment_processor?: Record<'run', string>[];
	downloaded_processor?: Record<'run', string>[];
	postprocessor?: (Record<'run' | 'mv', string> | 'rm')[];
	opt_args?: string[];
	override?: Record<string, any>;
}

export interface BiliType {
	id: number;
	children: BiliType[];
	name: string;
	desc: string;
}

export interface User {
	id: number;
	name: string;
	value: string;
	platform: string;
}

export interface BiliArchive {
	aid: number;
	bvid: string;
	title: string;
	cover: string;
	reject_reason: string;
	reject_reason_url: string;
	duration: number;
	desc: string;
	state: number;
	state_desc: string;
	dtime: number;
	ptime: number;
	ctime: number;
}

export interface BiliArchivePage {
	from_page: number;
	page_size: number;
	total: number;
	total_pages: number;
	fetched_pages: number;
	archives: BiliArchive[];
}

export interface FileList {
	key: number;
	name: string;
	updateTime: number;
	size: number;
}

export interface StreamerInfo {
	id: number;
	name: string;
	url: string;
	title: string;
	/** Unix 时间戳（秒），由后端 ts_seconds 序列化 */
	date: number;
	live_cover_path: string;
}
