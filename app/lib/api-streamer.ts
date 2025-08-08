// Fetcher implementation.
// The extra argument will be passed via the `arg` property of the 2nd parameter.
// In the example below, `arg` will be `'my_token'`
export async function sendRequest<T>(url: string, { arg }: {arg: T}) {
  // 获取认证信息
  const auth = typeof window !== 'undefined' ? localStorage.getItem('auth') : null;
  
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  
  // 如果存在认证信息，则添加到请求头
  if (auth) {
    headers['Authorization'] = `Basic ${auth}`;
  }
  
  const res =  await fetch((process.env.NEXT_PUBLIC_API_SERVER ?? '') + url, {
      method: 'POST',
      headers,
      body: JSON.stringify(arg)
  });
  if(!(res.status >=200 && res.status < 300)) {
    throw new Error(await res.text());
  }
  const data =  await res.json();
  if (!res.ok) {
    throw new Error(data.message);
  }
  return data;
}

export const fetcher = async (...args: any[]) => {
  // 获取认证信息
  const auth = typeof window !== 'undefined' ? localStorage.getItem('auth') : null;
  
  // 创建请求配置
  const init = args[1] || {};
  const headers: HeadersInit = init.headers || {};
  
  // 如果存在认证信息，则添加到请求头
  if (auth) {
    // 将headers转换为可修改的对象
    let headersObj: Record<string, string> = {};
    if (headers instanceof Headers) {
      headers.forEach((value, key) => {
        headersObj[key] = value;
      });
    } else if (Array.isArray(headers)) {
      headersObj = Object.fromEntries(headers);
    } else {
      headersObj = { ...headers };
    }
    headersObj['Authorization'] = `Basic ${auth}`;
    
    const res = await fetch((process.env.NEXT_PUBLIC_API_SERVER ?? '') + args[0], {
      ...init,
      headers: headersObj
    });
    if (!res.ok) {
      throw new Error(await res.text());
    }
    return res.json();
  }
  
  // 如果没有认证信息，直接发送请求
  const res = await fetch((process.env.NEXT_PUBLIC_API_SERVER ?? '') + args[0], {
    ...init,
    headers
  });
  if (!res.ok) {
    throw new Error(await res.text());
  }
  return res.json();
}

export const proxy = async (input: RequestInfo | URL, init?: RequestInit | undefined) => {
  // 获取认证信息
  const auth = typeof window !== 'undefined' ? localStorage.getItem('auth') : null;
  
  // 创建请求配置
  const requestInit = init || {};
  const headers: HeadersInit = requestInit.headers || {};
  
  // 如果存在认证信息，则添加到请求头
  if (auth) {
    // 将headers转换为可修改的对象
    let headersObj: Record<string, string> = {};
    if (headers instanceof Headers) {
      headers.forEach((value, key) => {
        headersObj[key] = value;
      });
    } else if (Array.isArray(headers)) {
      headersObj = Object.fromEntries(headers);
    } else {
      headersObj = { ...headers };
    }
    headersObj['Authorization'] = `Basic ${auth}`;
    
    const res = await fetch((process.env.NEXT_PUBLIC_API_SERVER ?? '') + input, {
      ...requestInit,
      headers: headersObj
    });
    if (!res.ok) {
      throw new Error(await res.text());
    }
    return res;
  }
  
  // 如果没有认证信息，直接发送请求
  const res = await fetch((process.env.NEXT_PUBLIC_API_SERVER ?? '') + input, {
    ...requestInit,
    headers
  });
  if (!res.ok) {
    throw new Error(await res.text());
  }
  return res;
}

export async function requestDelete<T>(url: string, { arg }: {arg: T}) {
  // 获取认证信息
  const auth = typeof window !== 'undefined' ? localStorage.getItem('auth') : null;
  
  const headers: Record<string, string> = {};
  
  // 如果存在认证信息，则添加到请求头
  if (auth) {
    headers['Authorization'] = `Basic ${auth}`;
  }
  
  const res =  await fetch(`${process.env.NEXT_PUBLIC_API_SERVER ?? ''}${url}/${arg}`, {
    method: 'DELETE',
    headers
  });
  if (!res.ok) {
    throw new Error(await res.text());
  }
  return res;
}

export async function put<T>(url: string, { arg }: {arg: T}) {
  // 获取认证信息
  const auth = typeof window !== 'undefined' ? localStorage.getItem('auth') : null;
  
  const headers: Record<string, string> = {
    'Content-Type': 'application/json'
  };
  
  // 如果存在认证信息，则添加到请求头
  if (auth) {
    headers['Authorization'] = `Basic ${auth}`;
  }
  
  const res =  await fetch(`${process.env.NEXT_PUBLIC_API_SERVER ?? ''}${url}`, {
    method: 'PUT',
    headers,
    body: JSON.stringify(arg)
  });
  if (!res.ok) {
    throw new Error(await res.text());
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

export interface LiveStreamerEntity {
	id: number;
	url: string;
	remark: string;
	filename: string;
	split_time?: number;
	split_size?: number;
	upload_id?: number;
	status?: string | React.ReactNode;
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

export interface FileList {
	key: number;
	name: string;
	updateTime: number;
	size: number;
}
