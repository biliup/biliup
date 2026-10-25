import {useMemo} from "react";
import useSWR from "swr";

import {
  BiliType,
  fetcher,
  LiveStreamerEntity,
  User
} from "./api-streamer";
import {boundedTasks} from "./bounded-tasks";
import {useMe} from "./use-me";


export default function useStreamers() {
  const { data, error, isLoading } = useSWR<LiveStreamerEntity[]>("/v1/streamers", fetcher);

  return {
    isLoading,
    streamers: data,
  };
}

const NO_USERS: any[] = [];

export function useBiliUsers(enabled = true) {
  // 账号列表归「编辑投稿模板」权限；没有这项权限的角色（只读观察者）不去请求，免得满屏 403
  const {can} = useMe();
  const {data, error, isLoading} = useSWR<User[]>(enabled && can("template.edit") ? "/v1/users" : null, fetcher);
  // Shared SWR key deduplicates profile requests across all mounted consumers.
  const {data: list, isLoading: profilesLoading} = useSWR(
    enabled && data?.length ? ['bili-user-profiles', data] : null,
    async ([, users]: [string, User[]]) => {
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
    const results = await boundedTasks(users, updateList);
    return results.flatMap(result => result.status === 'fulfilled' ? [result.value] : []);
  }, { dedupingInterval: 60_000, revalidateOnFocus: false });

  return {
    isLoading: isLoading || profilesLoading,
    isError: error,
    // 没有账号时直接派生为空列表,不必等 effect 里再 setState 一轮
    biliUsers: !enabled || !data?.length ? NO_USERS : list ?? NO_USERS,
  };
}

export function useTypeTree() {
  const {can} = useMe();
  const { data: archivePre, error, isLoading } = useSWR(can("upload.submit") ? "/bili/archive/pre" : null, fetcher);
  // 包一层 useMemo：下游（useTidNames、TemplateFields 的 Cascader）按引用做缓存，
  // 每次渲染都 map 出一个新数组的话，那些 useMemo 等于没生效
  const treeData = useMemo(() => archivePre?.data?.typelist.map((type: BiliType)=> {
    return {
      label: type.name,
      value: type.id,
      children: type.children
    };
  }), [archivePre]);
  return {
    isLoading,
    isError: error,
    typeTree: treeData,
  };
}

/** 分区树的一级分区（游戏 / 知识 / 生活…）；children 是二级分区，它的 id 才是投稿时用的 tid */
export interface TypeTreeNode {
  label: string;
  value: number;
  children: BiliType[];
}

/**
 * 投稿分区 tid → 分区名与完整路径（172 → 「网络游戏」/「游戏 / 网络游戏」）。
 * 数据源与投稿表单里的分区 Cascader 同为 /bili/archive/pre：后端拿本机第一个能登录的 cookie
 * 直接向 B 站要 typelist，所以不要硬编码 tid 表 —— B 站会增删分区、改名，而且返回的
 * 分区随账号类型而变（认证号、机构号能看到更多）。
 * 没有 upload.submit 权限、本机没有可用 cookie、接口失败时 map 为空，调用方回落到裸 tid。
 */
export function useTidNames() {
  const {typeTree} = useTypeTree();

  return useMemo(() => {
    const map = new Map<number, {name: string; path: string}>();
    for (const type of (typeTree ?? []) as TypeTreeNode[]) {
      for (const child of type.children ?? []) {
        map.set(child.id, {name: child.name, path: `${type.label} / ${child.name}`});
      }
    }
    return map;
  }, [typeTree]);
}
