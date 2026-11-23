(() => {
  const root = document.documentElement;
  const create = tag => document.createElementNS('http://www.w3.org/1999/xhtml', tag);
  /* In-app only */
  const source = root.querySelector(':scope > biliup-source');
  const data = new DOMParser().parseFromString(source.textContent, 'application/xml').documentElement;
  const platform = source.getAttribute('platform');
  const generic = platform !== 'Bilibili';
  source.remove();
  if (data.localName !== 'i' || data.querySelector('parsererror')) {
    root.appendChild(create('p')).textContent = '无法预览此弹幕 XML';
    return;
  }
  /* End in-app only */
  const kinds = { chat: '弹幕', super_chat: 'SC', guard_buy: '上舰', gift: '礼物', other: '其他' };
  /* In-app only */
  if (generic) Object.assign(kinds, { chat: '聊天', super_chat: '付费留言', guard_buy: '会员' });
  /* End in-app only */
  const number = value => value == null || value.trim() === '' || !Number.isFinite(+value) ? null : +value;
  const json = value => {
    try { return JSON.parse(value ?? 'null'); } catch { return null; }
  };
  function readGift(node) {
    const currency = node.getAttribute('coin_type') || json(node.getAttribute('raw'))?.data?.coin_type;
    return {
      name: node.getAttribute('giftname'),
      quantity: number(node.getAttribute('num')),
      amount: currency === 'silver' ? 0 : number(node.getAttribute('total_coin')),
    };
  }
  const giftTotal = (gifts, key) => gifts.some(gift => gift[key] === null)
    ? null : gifts.reduce((total, gift) => total + gift[key], 0);
  const giftLabel = ({ name, quantity }) =>
    (name || '—') + (quantity === null ? '' : ' x ' + quantity);
  const events = [...__DANMAKU_DATA__.children].filter(node => ['d', 's', 'o'].includes(node.localName))
    .map(node => {
      const attr = name => node.getAttribute(name);
      const kind = node.localName === 'd' ? 'chat' : attr('type');
      const type = node.localName !== 'o' && Object.hasOwn(kinds, kind) ? kind : 'other';
      const p = (attr('p') || '').split(',');
      const seconds = number(type === 'chat' ? p[0] : attr('ts'));
      const uid = attr('uid') || p[6] || '';
      const user = attr('user') || attr('username') || (uid !== '0' ? uid : '');
      const items = type === 'gift' ? [...node.querySelectorAll(':scope > item')].map(readGift) : [];
      const message = node.textContent.trim();
      // Keep thousandths of a yuan for exact sums and cross-type sorting.
      let content = type === 'super_chat' ? message : attr('giftname') || message;
      let quantity = null;
      let amount = null;
      if (type === 'gift') {
        const gifts = items.length ? items : [readGift(node)];
        content = gifts.map(giftLabel).join(', ');
        quantity = giftTotal(gifts, 'quantity');
        amount = giftTotal(gifts, 'amount');
      } else if (type === 'guard_buy') {
        quantity = number(attr('num'));
        amount = number(attr('price'));
      } else if (type === 'super_chat') {
        const originalPrice = number(attr('original_price'));
        amount = originalPrice === null ? number(attr('price')) : originalPrice * 1000;
      }
      return {
        node, type, uid, user, content, items, amount, quantity, raw: attr('raw'),
        duration: type === 'super_chat' ? number(attr('time')) : null,
        time: seconds === null ? null : Math.round(seconds * 1000),
        search: [user, uid, content, message, items.map(item => item.name).join('、')].join(' ').toLowerCase(),
      };
    });

  const view = create('main');
  view.className = 'danmaku-preview';
  let pageSize = 100;
  const filters = Object.entries(kinds).map(([kind, label]) =>
    `<label><input type="checkbox" value="${kind}"${kind === 'other' ? '' : ' checked="checked"'} /> ${label}</label>`).join('');
  const sizes = [20, 50, 100, 0].map(size =>
    `<button type="button" data-page-size="${size}" aria-pressed="${size === pageSize}">${size || '全部'}</button>`).join('');
  view.innerHTML = `<header><h1>录制弹幕</h1>
    <button id="theme-button" type="button" title="按自动、黑、白的顺序切换">主题 · 自动</button>
    </header><dl class="stats" aria-label="整份录制统计"></dl>
    <div class="filters">${filters}</div>
    <section class="time-filter" aria-label="弹幕热力图">
    <div class="heat-track"><div class="heat-plot"></div>
    <input id="time-start" type="range" min="0" step="any" aria-label="开始时间" />
    <input id="time-end" type="range" min="0" step="any" aria-label="结束时间" /></div>
    <div class="heat-labels"><span>开始 <output for="time-start"></output></span>
    <button type="button" id="clear-time" hidden="hidden">重置时间</button>
    <span>结束 <output for="time-end"></output></span></div>
    </section>
    <div class="table-controls">
    <input type="search" placeholder="搜索用户名、UID、弹幕或礼物" aria-label="搜索" />
    <label><input type="checkbox" id="show-details" /> 显示详细信息</label>
    <button type="button" id="clear-sender" hidden="hidden" title="清除发送者筛选"></button>
    <div class="page-size" role="group" aria-label="每页条数">${sizes}</div>
    </div>
    <table><thead><tr><th><button data-sort="time">出现时间</button></th><th>类型</th>
    <th>用户名</th><th>内容 / 礼物名</th><th><button data-sort="quantity">数量</button></th>
    <th><button data-sort="amount">金额（元）</button></th><th class="detail">详细信息</th>
    </tr></thead><tbody></tbody></table>
    <nav><button data-page="-1">上一页</button><span></span><button data-page="1">下一页</button></nav>`;
  root.append(view);
  const find = selector => view.querySelector(selector);
  const themeButton = find('#theme-button');
  const themes = [['system', '自动'], ['dark', '黑'], ['light', '白']];
  let themeIndex = 0;
  themeButton.addEventListener('click', () => {
    themeIndex = (themeIndex + 1) % themes.length;
    const [mode, label] = themes[themeIndex];
    root.setAttribute('data-theme', mode);
    themeButton.textContent = '主题 · ' + label;
  });
  let sort = 'time';
  let direction = 1;
  let page = 0;
  let timeRange = null;
  let senderUid = '';
  let moreRows = null;
  const heatTrack = find('.heat-track');
  const plot = find('.heat-plot');
  const startSlider = find('#time-start');
  const endSlider = find('#time-end');
  const clearTime = find('#clear-time');
  const clearSender = find('#clear-sender');

  /* In-app only */
  {
    const recordingStart = number(data.getAttribute('recording_start_time_ms'));
    if (!generic && recordingStart > 0) {
      for (const event of events) {
        if (event.type === 'chat' || event.time !== null) continue;
        const timestamp = number(event.node.getAttribute('timestamp'));
        if (!(timestamp > 0)) continue;
        const relative = Math.round((timestamp >= 1e12 ? timestamp : timestamp * 1000) - recordingStart);
        // Legacy timestamps rounded down to seconds can fall just before the start.
        if (relative >= -999) event.time = Math.max(relative, 0);
      }
    }
    const notice = create('aside');
    notice.className = generic ? 'preview-notice' : 'preview-notice warning';
    const icon = generic
      ? '<circle cx="12" cy="12" r="9"/><path d="M12 11v5m0-8h.01"/>'
      : '<path d="M10.3 4.4 2.7 18a2 2 0 0 0 1.7 3h15.2a2 2 0 0 0 1.7-3L13.7 4.4a2 2 0 0 0-3.4 0ZM12 9v4m0 4h.01"/>';
    const message = generic
      ? '<strong>仅预览</strong>　由 biliup 临时渲染此 XML。'
      : '此文件没有可直接使用的内嵌预览。当前为 biliup 提供的临时只读预览。';
    notice.innerHTML = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
      stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icon}</svg><span>${message}</span>`;
    view.prepend(notice);

    if (generic) {
      const types = new Set(events.map(event => event.type));
      const sender = events.some(event => event.user);
      const details = events.some(event => event.node.attributes.length > Number(event.node.hasAttribute('p')));
      view.setAttribute('data-hide-columns', [
        !sender && 'sender',
        types.size < 2 && 'type',
        !events.some(event => event.quantity !== null) && 'quantity',
        !events.some(event => event.amount !== null || event.duration !== null) && 'amount',
      ].filter(Boolean).join(' '));
      find('.filters').hidden = types.size < 2;
      for (const input of view.querySelectorAll('.filters input')) {
        input.parentElement.hidden = !types.has(input.value);
        if (types.size === 1 && types.has(input.value)) input.checked = true;
      }
      find('#show-details').parentElement.hidden = !details;
      const names = { Douyin: '抖音', Douyu: '斗鱼', Huya: '虎牙', Youtube: 'YouTube' };
      const name = names[platform] || (platform === 'General' ? '' : platform);
      find('h1').textContent = name ? name + ' 直播聊天' : '直播聊天';
      find('thead th:nth-child(3)').textContent = '发送者';
      if (!types.has('gift')) find('thead th:nth-child(4)').textContent = '内容';
      if (types.size === 1 && types.has('chat')) {
        find('input[type=search]').placeholder = sender ? '搜索发送者或内容' : '搜索聊天内容';
      }
    }
  }
  /* End in-app only */

  const timeParts = seconds => [Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60];
  const clock = ms => timeParts(Math.floor(ms / 1000)).map(value => String(value).padStart(2, '0')).join(':');
  const timestamp = (ms, precise = false) => clock(ms)
    + (precise || ms % 1000 ? '.' + String(ms % 1000).padStart(3, '0') : '');
  const show = value => value ?? '—';
  const money = new Intl.NumberFormat('zh-CN', {
    minimumFractionDigits: 2, maximumFractionDigits: 3, useGrouping: false,
  });
  const yuan = value => value === null ? '—' : money.format(value / 1000);

  const comments = [];
  let lastTime = 0;
  for (const { time, type } of events) {
    if (!Number.isSafeInteger(time) || time < 0) continue;
    lastTime = Math.max(lastTime, time);
    if (type === 'chat') comments.push(time);
  }
  const endTime = lastTime + 1;
  const peakWindow = 60000;
  comments.sort((a, b) => a - b);
  let peak;
  for (let left = 0, right = 0; right < comments.length; right++) {
    // Earliest start that still includes the rightmost event in [start, start + 60 s).
    const start = Math.max(0, comments[right] - peakWindow + 1);
    while (comments[left] < start) left++;
    const count = right - left + 1;
    if (!peak || count > peak.count) peak = { start, count };
  }

  function setTimeRange(start, end) {
    timeRange = start === 0 && end === endTime ? null : [start, end];
    for (const [slider, value] of [[startSlider, start], [endSlider, end]]) {
      slider.value = value;
      const time = Math.min(value, lastTime);
      const label = find(`output[for="${slider.id}"]`);
      label.textContent = clock(time);
      label.title = timestamp(time);
      slider.setAttribute('aria-valuetext', label.title);
    }
    plot.style.setProperty('--from', start / endTime * 100 + '%');
    plot.style.setProperty('--to', end / endTime * 100 + '%');
    clearTime.hidden = !timeRange;
  }

  function drawHeatmap() {
    const step = Math.ceil(endTime / 240000) * 1000;
    const cells = Array(Math.ceil(endTime / step)).fill(0);
    for (const time of comments) cells[Math.floor(time / step)]++;
    const max = Math.max(1, ...cells);
    const points = ['0% 100%'];
    plot.innerHTML = cells.map((count, index) => {
      const start = index * step, stop = Math.min(start + step, endTime);
      const x = ((start + stop) / (2 * endTime) * 100).toFixed(2);
      const y = (100 - 93 * count / max).toFixed(2);
      points.push(x + '% ' + y + '%');
      const title = `${timestamp(start)}–${timestamp(Math.min(stop, lastTime))} · ${count} 条普通弹幕`;
      return `<span style="flex:${stop - start}" title="${title}"></span>`;
    }).join('');
    points.push('100% 100%');
    plot.style.setProperty('--profile', `polygon(${points.join(',')})`);
    startSlider.max = endSlider.max = endTime;
    setTimeRange(0, endTime);
  }

  heatTrack.addEventListener('input', ({ target }) => {
    const start = Math.round(+startSlider.value), end = Math.round(+endSlider.value);
    if (target === startSlider) setTimeRange(Math.min(start, end - 1), end);
    else setTimeRange(start, Math.max(start + 1, end));
  });
  heatTrack.addEventListener('change', () => {
    page = 0;
    draw();
  });
  clearTime.addEventListener('click', () => {
    setTimeRange(0, endTime);
    page = 0;
    draw();
  });

  function drawStats() {
    const chat = events.filter(event => event.type === 'chat');
    const sc = events.filter(event => event.type === 'super_chat');
    const guard = events.filter(event => event.type === 'guard_buy');
    const gifts = events.filter(event => event.type === 'gift');
    const giftItems = gifts.flatMap(event => event.items.length ? event.items : [event]);
    const people = rows => new Set(rows.map(row => row.uid).filter(uid => uid && uid !== '0')).size;
    const knownTotal = (rows, key) => {
      const values = rows.map(row => row[key]).filter(value => value !== null);
      return values.length ? values.reduce((total, value) => total + value, 0) : null;
    };
    const amountText = rows => {
      const amount = knownTotal(rows, 'amount');
      return amount === null ? '—' : yuan(amount) + ' 元';
    };
    const giftCount = giftItems.length ? knownTotal(giftItems, 'quantity') : 0;
    const giftPrefix = giftItems.some(item => item.quantity === null) ? '已知 ' : '';
    const stats = [
      ['普通弹幕', `${chat.length} 条`, `已识别 ${people([...chat, ...sc])} 人发言`],
      ['弹幕峰值', peak ? `<button type="button" class="text-button" data-peak-time="${peak.start}" title="筛选峰值所在的 60 秒">${peak.count} 条/分钟</button>` : '—',
        peak && clock(peak.start) + ' 起'],
      ['礼物', giftCount === null ? '—' : `${giftPrefix}${giftCount} 件`, `已识别 ${people(gifts)} 人送礼`],
      ['SC', `${sc.length} 条`, sc.length && amountText(sc)],
      ['上舰', `${guard.length} 笔`, guard.length && amountText(guard)],
      ['已知金额合计', amountText([...giftItems, ...sc, ...guard]), '礼物、SC 和上舰'],
    ];
    /* In-app only */
    if (generic) {
      if (!people([...chat, ...sc])) stats[0][2] = '';
      stats[0][0] = '聊天消息';
      stats[1][0] = '聊天峰值';
      stats[3][0] = kinds.super_chat;
      stats[4][0] = kinds.guard_buy;
      stats[5][2] = '';
      const present = [gifts.length, sc.length, guard.length, stats[5][1] !== '—'];
      stats.splice(2, present.length, ...stats.slice(2).filter((_, index) => present[index]));
    }
    /* End in-app only */
    find('.stats').innerHTML = stats.map(([label, value, note]) =>
      `<div><dt>${label}</dt><dd>${value}${note ? `<small>${note}</small>` : ''}</dd></div>`).join('');
  }

  function durationText(seconds) {
    if (seconds <= 60) return seconds + ' 秒';
    const units = ['小时', '分钟', '秒'];
    return timeParts(seconds).map((value, index) => value ? value + ' ' + units[index] : '')
      .filter(Boolean).join(' ');
  }

  function emoteNode(url, fallback) {
    if (typeof url !== 'string' || !/^(?:https?:)?\/\/(?:[\w-]+\.)*(?:hdslb\.com|biliimg\.com)\//i.test(url)) {
      return fallback;
    }
    const image = create('img');
    image.src = url.replace(/^(?:https?:)?\/\//i, 'https://');
    image.alt = fallback;
    image.loading = 'lazy';
    image.referrerPolicy = 'no-referrer';
    image.addEventListener('error', () => image.replaceWith(fallback), { once: true });
    return image;
  }

  function drawEmotes(cell, content, { node, raw }) {
    let emote = node.getAttribute('emoji_url');
    let images;
    if (!emote && node.hasAttribute('emoji_1_text')) {
      images = new Map();
      for (let index = 1; ; index++) {
        const text = node.getAttribute(`emoji_${index}_text`);
        if (!text) break;
        images.set(text, node.getAttribute(`emoji_${index}_url`));
      }
    }
    if (!emote && !images?.size && raw) {
      const packet = json(raw);
      const meta = (Array.isArray(packet) ? packet : packet?.info)?.[0];
      const extra = json(meta?.[15]?.extra);
      emote = meta?.[13]?.url || extra?.emoticon?.url || packet?.data?.emoji_img_url;
      if (!emote && extra?.emots) {
        images = new Map(Object.entries(extra.emots).map(([text, image]) => [text, image?.url]));
      }
    }
    if (emote) {
      cell.replaceChildren(emoteNode(emote, content));
    } else if (images?.size) {
      cell.replaceChildren(...content.split(/(\[[^\]]+\])/).map(part => emoteNode(images.get(part), part)));
    }
  }

  function drawRows(tbody, entries, showDetails) {
    for (const event of entries) {
      const row = tbody.insertRow();
      const time = event.time === null ? '—' : clock(event.time);
      const duration = event.duration === null ? '' : ' (' + durationText(event.duration) + ')';
      const quantity = event.quantity !== null && event.type === 'guard_buy'
        ? `${event.quantity} 个月` : show(event.quantity);
      const content = event.content || '—';
      const values = [time, kinds[event.type], event.user || '—', content, quantity, yuan(event.amount) + duration];
      for (const value of values) row.insertCell().textContent = value;
      if (event.uid && event.uid !== '0') {
        const sender = create('button');
        sender.type = 'button';
        sender.className = 'text-button';
        sender.dataset.uid = event.uid;
        sender.title = '按 UID ' + event.uid + ' 筛选';
        sender.textContent = event.user;
        row.cells[2].replaceChildren(sender);
      }
      if (event.type === 'chat') drawEmotes(row.cells[3], content, event);
      if (event.time !== null) row.cells[0].title = timestamp(event.time, true);

      const detailCell = row.insertCell();
      detailCell.className = 'detail';
      if (!showDetails) continue;
      const extra = [...event.node.attributes].filter(attr => !['p', 'raw'].includes(attr.name));
      if (!extra.length && !event.items.length && !event.raw) continue;
      const details = detailCell.appendChild(create('details'));
      details.innerHTML = '<summary>详情</summary>';
      details.addEventListener('toggle', () => {
        const lines = event.items.map(item =>
          `${item.name} × ${show(item.quantity)} · 金额（元） ${yuan(item.amount)}`);
        lines.push(...extra.map(attr => `${attr.name} = ${attr.value}`));
        if (event.raw) lines.push(event.raw);
        details.appendChild(create('pre')).textContent = lines.join('\n');
      }, { once: true });
    }
  }

  function draw() {
    moreRows?.disconnect();
    moreRows = null;
    const showDetails = find('#show-details').checked;
    const types = new Set([...view.querySelectorAll('.filters input:checked')].map(input => input.value));
    const query = find('[type=search]').value.trim().toLowerCase();
    const visible = events.filter(event => types.has(event.type) && event.search.includes(query)
      && (!senderUid || event.uid === senderUid)
      && (!timeRange || (event.time !== null && event.time >= timeRange[0] && event.time < timeRange[1])));
    visible.sort((a, b) => {
      const x = a[sort], y = b[sort];
      if (x === null || y === null) return (x === null) - (y === null);
      return direction * (x - y);
    });
    const tbody = find('tbody');
    const status = find('nav span');
    tbody.replaceChildren();
    const entries = pageSize ? visible.slice(page * pageSize, (page + 1) * pageSize) : visible;
    const chunk = 200;
    let rendered = pageSize ? entries.length : Math.min(chunk, entries.length);
    drawRows(tbody, entries.slice(0, rendered), showDetails);
    if (rendered < entries.length) {
      const observer = new IntersectionObserver(([entry]) => {
        if (observer !== moreRows || !entry.isIntersecting) return;
        const end = Math.min(rendered + chunk, entries.length);
        drawRows(tbody, entries.slice(rendered, end), showDetails);
        rendered = end;
        observer.unobserve(status);
        if (rendered < entries.length) observer.observe(status);
      }, { rootMargin: '320px 0px' });
      moreRows = observer;
      observer.observe(status);
    }
    for (const button of view.querySelectorAll('[data-sort]')) {
      const order = button.dataset.sort === sort ? direction === 1 ? 'ascending' : 'descending' : 'none';
      button.parentElement.setAttribute('aria-sort', order);
    }
    const pages = pageSize ? Math.max(1, Math.ceil(visible.length / pageSize)) : 1;
    status.textContent = `${visible.length} 条` + (pageSize ? ` · ${page + 1}/${pages} 页` : '');
    const previous = find('[data-page="-1"]');
    const next = find('[data-page="1"]');
    previous.hidden = next.hidden = pageSize === 0;
    previous.disabled = page === 0;
    next.disabled = page + 1 >= pages;
  }

  view.addEventListener('input', ({ target }) => {
    if (heatTrack.contains(target)) return;
    if (target.id !== 'show-details') page = 0;
    draw();
  });
  view.addEventListener('click', ({ target }) => {
    const { sort: nextSort, page: advance, pageSize: nextSize, uid, peakTime } = target.dataset;
    if (peakTime !== undefined) {
      const start = +peakTime;
      setTimeRange(start, Math.min(start + peakWindow, endTime));
      page = 0;
      draw();
      return;
    }
    if (uid || target === clearSender) {
      senderUid = uid || '';
      clearSender.hidden = !uid;
      clearSender.textContent = uid ? '发送者 · ' + target.textContent + ' ×' : '';
      page = 0;
    } else if (nextSort) {
      direction = sort === nextSort ? -direction : 1;
      sort = nextSort;
      page = 0;
    } else if (nextSize !== undefined && pageSize !== +nextSize) {
      pageSize = +nextSize;
      page = 0;
      find('.page-size [aria-pressed=true]').setAttribute('aria-pressed', 'false');
      target.setAttribute('aria-pressed', 'true');
    } else if (advance) {
      page += +advance;
    } else {
      return;
    }
    draw();
  });
  drawHeatmap();
  drawStats();
  draw();
})();
