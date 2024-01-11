export const responsiveMap = {
    xs: '(max-width: 575px)',
    sm: '(min-width: 576px)',
    md: '(min-width: 768px)',
    lg: '(min-width: 992px)',
    xl: '(min-width: 1200px)',
    xxl: '(min-width: 1600px)',
};


export interface RegisterMediaQueryOption {
    match?: (e: MediaQueryList | MediaQueryListEvent) => void;
    unmatch?: (e: MediaQueryList | MediaQueryListEvent) => void;
    callInInit?: boolean
}

/**
 * register matchFn and unMatchFn callback while media query
 * @param {string} media media string
 * @param {object} param param object
 * @returns function
 */
export const registerMediaQuery = (media: string, { match, unmatch, callInInit = true }: RegisterMediaQueryOption): () => void => {
    if (typeof window !== 'undefined') {
        const mediaQueryList = window.matchMedia(media);
        const handlerMediaChange = function(e: MediaQueryList | MediaQueryListEvent): void {
            if (e.matches) {
                match && match(e);
            } else {
                unmatch && unmatch(e);
            }
        }
        callInInit && handlerMediaChange(mediaQueryList);
        if (Object.prototype.hasOwnProperty.call(mediaQueryList, 'addEventListener')) {
            mediaQueryList.addEventListener('change', handlerMediaChange);
            return (): void => mediaQueryList.removeEventListener('change', handlerMediaChange);
        }
        mediaQueryList.addListener(handlerMediaChange);
        return (): void => mediaQueryList.removeListener(handlerMediaChange);
    }
    return () => undefined;
};

export const humDate = function (time: number) {
    const updateTime = new Date(time * 1000)
    //日期
    const DD = String(updateTime.getDate()).padStart(2, '0'); // 获取日
    const MM = String(updateTime.getMonth() + 1).padStart(2, '0'); //获取月份，1 月为 0
    const yyyy = updateTime.getFullYear(); // 获取年

    // 时间
    const hh =  String(updateTime.getHours()).padStart(2, '0');       //获取当前小时数(0-23)
    const mm = String(updateTime.getMinutes()).padStart(2, '0');     //获取当前分钟数(0-59)
    const ss = String(updateTime.getSeconds()).padStart(2, '0');
    return yyyy + '-' + MM + '-' + DD + ' ' + hh + ':' + mm + ':' + ss;
}