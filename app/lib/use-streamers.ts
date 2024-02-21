import useSWR from "swr";
import useSWRMutation from 'swr/mutation';

import {
  addTemplate,
  BiliType,
  fetcher,
  LiveStreamerEntity, proxy,
  requestDelete,
  sendRequest,
  User
} from "./api-streamer";
import React, {useEffect, useState} from "react";


export default function useStreamers() {
  const { data, error, isLoading } = useSWR<LiveStreamerEntity[]>("/v1/streamers", fetcher);

  return {
    isLoading,
    streamers: data,
  };
}

export function useBiliUsers() {
  const {data, error, isLoading} = useSWR<User[]>("/v1/users", fetcher);
  const [list, setList] = useState<any[]>([]);
  useEffect(() => {
    if (!data || data.length === 0) {
      setList([]);
      return;
    }
  
    const updateList = async (item: User) => {
      const res = await fetcher(`/bili/space/myinfo?user=${item.value}`, undefined);
      const pRes = await proxy(`/bili/proxy?url=${res.data.face}`);
      const myBlob = await pRes.blob();
  
      return {
        ...item,
        name: res.data.name,
        face: URL.createObjectURL(myBlob),
      };
    };
  
    const updateData = async (data: User[]) => {
      const updatedList = await Promise.all(data.map(updateList));
      setList(updatedList);
    };
  
    updateData(data);
  }, [data]);
  
  return {
    isLoading,
    isError: error,
    biliUsers: list,
  };
}

export function useTypeTree() {
  const { data: archivePre, error, isLoading } = useSWR("/bili/archive/pre", fetcher);
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
