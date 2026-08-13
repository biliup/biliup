import useSWR from "swr";

import {
  BiliType,
  fetcher,
  LiveStreamerEntity,
  User
} from "./api-streamer";
import {useEffect, useState} from "react";


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
      setList([])
      return;
    }
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
    Promise.all(data.map(updateList)).then(setList);
  }, [data])

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
