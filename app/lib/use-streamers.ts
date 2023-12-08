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
    if (data?.length === 0) {
      setList([]);
    }
    data?.forEach(item => {
      (async () => {
        const res = await fetcher(`/bili/space/myinfo?user=${item.value}`, undefined)
        const pRes = await proxy(`/bili/proxy?url=${res.data.face}`)
        const myBlob = await pRes.blob()
        const newList = data.map(value => {
          if (value.id === item.id) {
            return {
              ...value,
              name: res.data.name,
              face: URL.createObjectURL(myBlob),
            };
          }
          return value;
        });
        setList(newList);
      })()
    })
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
