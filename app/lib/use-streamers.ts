import useSWR from "swr";

import {
  BiliType,
  fetcher,
  LiveStreamerEntity,
  User
} from "./api-streamer";
import {useEffect, useState} from "react";
import {useMe} from "./use-me";


export default function useStreamers() {
  const { data, error, isLoading } = useSWR<LiveStreamerEntity[]>("/v1/streamers", fetcher);

  return {
    isLoading,
    streamers: data,
  };
}

const NO_USERS: any[] = [];

export function useBiliUsers() {
  // 账号列表归「编辑投稿模板」权限；没有这项权限的角色（只读观察者）不去请求，免得满屏 403
  const {can} = useMe();
  const {data, error, isLoading} = useSWR<User[]>(can("template.edit") ? "/v1/users" : null, fetcher);
  const [list, setList] = useState<any[]>(NO_USERS);
  useEffect(() => {
    if (!data) return;
    const updateList = async (item: User) => {
      try {
        const res = await fetcher(`/v1/users/${item.id}`, undefined);
        return {
          ...item,
          name: res.data.name,
          face: res.data?.face || "/noface.jpg",
        };
      } catch (error) {
        console.error(error);
        return {
          ...item,
          name: "Cookie已失效",
          face: "/noface.jpg",
        };
      }
    };
    // data 为空数组时这里解析为 [],顺带把上一批账号的补全结果清掉,
    // 之后再有账号时不会先闪一下旧数据
    Promise.all(data.map(updateList)).then(setList);
  }, [data])

  return {
    isLoading,
    isError: error,
    // 没有账号时直接派生为空列表,不必等 effect 里再 setState 一轮
    biliUsers: !data || data.length === 0 ? NO_USERS : list,
  };
}

export function useTypeTree() {
  const {can} = useMe();
  const { data: archivePre, error, isLoading } = useSWR(can("upload.submit") ? "/bili/archive/pre" : null, fetcher);
  const treeData = archivePre?.data?.typelist.map((type: BiliType)=> {
    return {
      label: type.name,
      value: type.id,
      children: type.children
    };
  });
  return {
    isLoading,
    isError: error,
    typeTree: treeData,
  };
}
