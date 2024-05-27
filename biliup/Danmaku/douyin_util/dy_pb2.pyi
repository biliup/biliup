from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class CommentTypeTag(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = []
    COMMENTTYPETAGUNKNOWN: _ClassVar[CommentTypeTag]
    COMMENTTYPETAGSTAR: _ClassVar[CommentTypeTag]
COMMENTTYPETAGUNKNOWN: CommentTypeTag
COMMENTTYPETAGSTAR: CommentTypeTag

class Response(_message.Message):
    __slots__ = ["messagesList", "cursor", "fetchInterval", "now", "internalExt", "fetchType", "routeParams", "heartbeatDuration", "needAck", "pushServer", "liveCursor", "historyNoMore"]
    class RouteParamsEntry(_message.Message):
        __slots__ = ["key", "value"]
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    MESSAGESLIST_FIELD_NUMBER: _ClassVar[int]
    CURSOR_FIELD_NUMBER: _ClassVar[int]
    FETCHINTERVAL_FIELD_NUMBER: _ClassVar[int]
    NOW_FIELD_NUMBER: _ClassVar[int]
    INTERNALEXT_FIELD_NUMBER: _ClassVar[int]
    FETCHTYPE_FIELD_NUMBER: _ClassVar[int]
    ROUTEPARAMS_FIELD_NUMBER: _ClassVar[int]
    HEARTBEATDURATION_FIELD_NUMBER: _ClassVar[int]
    NEEDACK_FIELD_NUMBER: _ClassVar[int]
    PUSHSERVER_FIELD_NUMBER: _ClassVar[int]
    LIVECURSOR_FIELD_NUMBER: _ClassVar[int]
    HISTORYNOMORE_FIELD_NUMBER: _ClassVar[int]
    messagesList: _containers.RepeatedCompositeFieldContainer[Message]
    cursor: str
    fetchInterval: int
    now: int
    internalExt: str
    fetchType: int
    routeParams: _containers.ScalarMap[str, str]
    heartbeatDuration: int
    needAck: bool
    pushServer: str
    liveCursor: str
    historyNoMore: bool
    def __init__(self, messagesList: _Optional[_Iterable[_Union[Message, _Mapping]]] = ..., cursor: _Optional[str] = ..., fetchInterval: _Optional[int] = ..., now: _Optional[int] = ..., internalExt: _Optional[str] = ..., fetchType: _Optional[int] = ..., routeParams: _Optional[_Mapping[str, str]] = ..., heartbeatDuration: _Optional[int] = ..., needAck: bool = ..., pushServer: _Optional[str] = ..., liveCursor: _Optional[str] = ..., historyNoMore: bool = ...) -> None: ...

class Message(_message.Message):
    __slots__ = ["method", "payload", "msgId", "msgType", "offset", "needWrdsStore", "wrdsVersion", "wrdsSubKey"]
    METHOD_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    MSGID_FIELD_NUMBER: _ClassVar[int]
    MSGTYPE_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    NEEDWRDSSTORE_FIELD_NUMBER: _ClassVar[int]
    WRDSVERSION_FIELD_NUMBER: _ClassVar[int]
    WRDSSUBKEY_FIELD_NUMBER: _ClassVar[int]
    method: str
    payload: bytes
    msgId: int
    msgType: int
    offset: int
    needWrdsStore: bool
    wrdsVersion: int
    wrdsSubKey: str
    def __init__(self, method: _Optional[str] = ..., payload: _Optional[bytes] = ..., msgId: _Optional[int] = ..., msgType: _Optional[int] = ..., offset: _Optional[int] = ..., needWrdsStore: bool = ..., wrdsVersion: _Optional[int] = ..., wrdsSubKey: _Optional[str] = ...) -> None: ...

class ChatMessage(_message.Message):
    __slots__ = ["common", "user", "content", "visibleToSender", "backgroundImage", "fullScreenTextColor", "backgroundImageV2", "publicAreaCommon", "giftImage", "agreeMsgId", "priorityLevel", "landscapeAreaCommon", "eventTime", "sendReview", "fromIntercom", "intercomHideUserCard", "chatBy", "individualChatPriority", "rtfContent"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    VISIBLETOSENDER_FIELD_NUMBER: _ClassVar[int]
    BACKGROUNDIMAGE_FIELD_NUMBER: _ClassVar[int]
    FULLSCREENTEXTCOLOR_FIELD_NUMBER: _ClassVar[int]
    BACKGROUNDIMAGEV2_FIELD_NUMBER: _ClassVar[int]
    PUBLICAREACOMMON_FIELD_NUMBER: _ClassVar[int]
    GIFTIMAGE_FIELD_NUMBER: _ClassVar[int]
    AGREEMSGID_FIELD_NUMBER: _ClassVar[int]
    PRIORITYLEVEL_FIELD_NUMBER: _ClassVar[int]
    LANDSCAPEAREACOMMON_FIELD_NUMBER: _ClassVar[int]
    EVENTTIME_FIELD_NUMBER: _ClassVar[int]
    SENDREVIEW_FIELD_NUMBER: _ClassVar[int]
    FROMINTERCOM_FIELD_NUMBER: _ClassVar[int]
    INTERCOMHIDEUSERCARD_FIELD_NUMBER: _ClassVar[int]
    CHATBY_FIELD_NUMBER: _ClassVar[int]
    INDIVIDUALCHATPRIORITY_FIELD_NUMBER: _ClassVar[int]
    RTFCONTENT_FIELD_NUMBER: _ClassVar[int]
    common: Common
    user: User
    content: str
    visibleToSender: bool
    backgroundImage: Image
    fullScreenTextColor: str
    backgroundImageV2: Image
    publicAreaCommon: PublicAreaCommon
    giftImage: Image
    agreeMsgId: int
    priorityLevel: int
    landscapeAreaCommon: LandscapeAreaCommon
    eventTime: int
    sendReview: bool
    fromIntercom: bool
    intercomHideUserCard: bool
    chatBy: str
    individualChatPriority: int
    rtfContent: Text
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., user: _Optional[_Union[User, _Mapping]] = ..., content: _Optional[str] = ..., visibleToSender: bool = ..., backgroundImage: _Optional[_Union[Image, _Mapping]] = ..., fullScreenTextColor: _Optional[str] = ..., backgroundImageV2: _Optional[_Union[Image, _Mapping]] = ..., publicAreaCommon: _Optional[_Union[PublicAreaCommon, _Mapping]] = ..., giftImage: _Optional[_Union[Image, _Mapping]] = ..., agreeMsgId: _Optional[int] = ..., priorityLevel: _Optional[int] = ..., landscapeAreaCommon: _Optional[_Union[LandscapeAreaCommon, _Mapping]] = ..., eventTime: _Optional[int] = ..., sendReview: bool = ..., fromIntercom: bool = ..., intercomHideUserCard: bool = ..., chatBy: _Optional[str] = ..., individualChatPriority: _Optional[int] = ..., rtfContent: _Optional[_Union[Text, _Mapping]] = ...) -> None: ...

class LandscapeAreaCommon(_message.Message):
    __slots__ = ["showHead", "showNickname", "showFontColor", "colorValueList", "commentTypeTagsList"]
    SHOWHEAD_FIELD_NUMBER: _ClassVar[int]
    SHOWNICKNAME_FIELD_NUMBER: _ClassVar[int]
    SHOWFONTCOLOR_FIELD_NUMBER: _ClassVar[int]
    COLORVALUELIST_FIELD_NUMBER: _ClassVar[int]
    COMMENTTYPETAGSLIST_FIELD_NUMBER: _ClassVar[int]
    showHead: bool
    showNickname: bool
    showFontColor: bool
    colorValueList: _containers.RepeatedScalarFieldContainer[str]
    commentTypeTagsList: _containers.RepeatedScalarFieldContainer[CommentTypeTag]
    def __init__(self, showHead: bool = ..., showNickname: bool = ..., showFontColor: bool = ..., colorValueList: _Optional[_Iterable[str]] = ..., commentTypeTagsList: _Optional[_Iterable[_Union[CommentTypeTag, str]]] = ...) -> None: ...

class RoomUserSeqMessage(_message.Message):
    __slots__ = ["common", "ranksList", "total", "popStr", "seatsList", "popularity", "totalUser", "totalUserStr", "totalStr", "onlineUserForAnchor", "totalPvForAnchor", "upRightStatsStr", "upRightStatsStrComplete"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    RANKSLIST_FIELD_NUMBER: _ClassVar[int]
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    POPSTR_FIELD_NUMBER: _ClassVar[int]
    SEATSLIST_FIELD_NUMBER: _ClassVar[int]
    POPULARITY_FIELD_NUMBER: _ClassVar[int]
    TOTALUSER_FIELD_NUMBER: _ClassVar[int]
    TOTALUSERSTR_FIELD_NUMBER: _ClassVar[int]
    TOTALSTR_FIELD_NUMBER: _ClassVar[int]
    ONLINEUSERFORANCHOR_FIELD_NUMBER: _ClassVar[int]
    TOTALPVFORANCHOR_FIELD_NUMBER: _ClassVar[int]
    UPRIGHTSTATSSTR_FIELD_NUMBER: _ClassVar[int]
    UPRIGHTSTATSSTRCOMPLETE_FIELD_NUMBER: _ClassVar[int]
    common: Common
    ranksList: _containers.RepeatedCompositeFieldContainer[RoomUserSeqMessageContributor]
    total: int
    popStr: str
    seatsList: _containers.RepeatedCompositeFieldContainer[RoomUserSeqMessageContributor]
    popularity: int
    totalUser: int
    totalUserStr: str
    totalStr: str
    onlineUserForAnchor: str
    totalPvForAnchor: str
    upRightStatsStr: str
    upRightStatsStrComplete: str
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., ranksList: _Optional[_Iterable[_Union[RoomUserSeqMessageContributor, _Mapping]]] = ..., total: _Optional[int] = ..., popStr: _Optional[str] = ..., seatsList: _Optional[_Iterable[_Union[RoomUserSeqMessageContributor, _Mapping]]] = ..., popularity: _Optional[int] = ..., totalUser: _Optional[int] = ..., totalUserStr: _Optional[str] = ..., totalStr: _Optional[str] = ..., onlineUserForAnchor: _Optional[str] = ..., totalPvForAnchor: _Optional[str] = ..., upRightStatsStr: _Optional[str] = ..., upRightStatsStrComplete: _Optional[str] = ...) -> None: ...

class CommonTextMessage(_message.Message):
    __slots__ = ["common", "user", "scene"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    common: Common
    user: User
    scene: str
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., user: _Optional[_Union[User, _Mapping]] = ..., scene: _Optional[str] = ...) -> None: ...

class UpdateFanTicketMessage(_message.Message):
    __slots__ = ["common", "roomFanTicketCountText", "roomFanTicketCount", "forceUpdate"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    ROOMFANTICKETCOUNTTEXT_FIELD_NUMBER: _ClassVar[int]
    ROOMFANTICKETCOUNT_FIELD_NUMBER: _ClassVar[int]
    FORCEUPDATE_FIELD_NUMBER: _ClassVar[int]
    common: Common
    roomFanTicketCountText: str
    roomFanTicketCount: int
    forceUpdate: bool
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., roomFanTicketCountText: _Optional[str] = ..., roomFanTicketCount: _Optional[int] = ..., forceUpdate: bool = ...) -> None: ...

class RoomUserSeqMessageContributor(_message.Message):
    __slots__ = ["score", "user", "rank", "delta", "isHidden", "scoreDescription", "exactlyScore"]
    SCORE_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    RANK_FIELD_NUMBER: _ClassVar[int]
    DELTA_FIELD_NUMBER: _ClassVar[int]
    ISHIDDEN_FIELD_NUMBER: _ClassVar[int]
    SCOREDESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    EXACTLYSCORE_FIELD_NUMBER: _ClassVar[int]
    score: int
    user: User
    rank: int
    delta: int
    isHidden: bool
    scoreDescription: str
    exactlyScore: str
    def __init__(self, score: _Optional[int] = ..., user: _Optional[_Union[User, _Mapping]] = ..., rank: _Optional[int] = ..., delta: _Optional[int] = ..., isHidden: bool = ..., scoreDescription: _Optional[str] = ..., exactlyScore: _Optional[str] = ...) -> None: ...

class GiftMessage(_message.Message):
    __slots__ = ["common", "giftId", "fanTicketCount", "groupCount", "repeatCount", "comboCount", "user", "toUser", "repeatEnd", "textEffect", "groupId", "incomeTaskgifts", "roomFanTicketCount", "priority", "gift", "logId", "sendType", "publicAreaCommon", "trayDisplayText", "bannedDisplayEffects", "displayForSelf", "interactGiftInfo", "diyItemInfo", "minAssetSetList", "totalCount", "clientGiftSource", "toUserIdsList", "sendTime", "forceDisplayEffects", "traceId", "effectDisplayTs"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    GIFTID_FIELD_NUMBER: _ClassVar[int]
    FANTICKETCOUNT_FIELD_NUMBER: _ClassVar[int]
    GROUPCOUNT_FIELD_NUMBER: _ClassVar[int]
    REPEATCOUNT_FIELD_NUMBER: _ClassVar[int]
    COMBOCOUNT_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    TOUSER_FIELD_NUMBER: _ClassVar[int]
    REPEATEND_FIELD_NUMBER: _ClassVar[int]
    TEXTEFFECT_FIELD_NUMBER: _ClassVar[int]
    GROUPID_FIELD_NUMBER: _ClassVar[int]
    INCOMETASKGIFTS_FIELD_NUMBER: _ClassVar[int]
    ROOMFANTICKETCOUNT_FIELD_NUMBER: _ClassVar[int]
    PRIORITY_FIELD_NUMBER: _ClassVar[int]
    GIFT_FIELD_NUMBER: _ClassVar[int]
    LOGID_FIELD_NUMBER: _ClassVar[int]
    SENDTYPE_FIELD_NUMBER: _ClassVar[int]
    PUBLICAREACOMMON_FIELD_NUMBER: _ClassVar[int]
    TRAYDISPLAYTEXT_FIELD_NUMBER: _ClassVar[int]
    BANNEDDISPLAYEFFECTS_FIELD_NUMBER: _ClassVar[int]
    DISPLAYFORSELF_FIELD_NUMBER: _ClassVar[int]
    INTERACTGIFTINFO_FIELD_NUMBER: _ClassVar[int]
    DIYITEMINFO_FIELD_NUMBER: _ClassVar[int]
    MINASSETSETLIST_FIELD_NUMBER: _ClassVar[int]
    TOTALCOUNT_FIELD_NUMBER: _ClassVar[int]
    CLIENTGIFTSOURCE_FIELD_NUMBER: _ClassVar[int]
    TOUSERIDSLIST_FIELD_NUMBER: _ClassVar[int]
    SENDTIME_FIELD_NUMBER: _ClassVar[int]
    FORCEDISPLAYEFFECTS_FIELD_NUMBER: _ClassVar[int]
    TRACEID_FIELD_NUMBER: _ClassVar[int]
    EFFECTDISPLAYTS_FIELD_NUMBER: _ClassVar[int]
    common: Common
    giftId: int
    fanTicketCount: int
    groupCount: int
    repeatCount: int
    comboCount: int
    user: User
    toUser: User
    repeatEnd: int
    textEffect: TextEffect
    groupId: int
    incomeTaskgifts: int
    roomFanTicketCount: int
    priority: GiftIMPriority
    gift: GiftStruct
    logId: str
    sendType: int
    publicAreaCommon: PublicAreaCommon
    trayDisplayText: Text
    bannedDisplayEffects: int
    displayForSelf: bool
    interactGiftInfo: str
    diyItemInfo: str
    minAssetSetList: _containers.RepeatedScalarFieldContainer[int]
    totalCount: int
    clientGiftSource: int
    toUserIdsList: _containers.RepeatedScalarFieldContainer[int]
    sendTime: int
    forceDisplayEffects: int
    traceId: str
    effectDisplayTs: int
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., giftId: _Optional[int] = ..., fanTicketCount: _Optional[int] = ..., groupCount: _Optional[int] = ..., repeatCount: _Optional[int] = ..., comboCount: _Optional[int] = ..., user: _Optional[_Union[User, _Mapping]] = ..., toUser: _Optional[_Union[User, _Mapping]] = ..., repeatEnd: _Optional[int] = ..., textEffect: _Optional[_Union[TextEffect, _Mapping]] = ..., groupId: _Optional[int] = ..., incomeTaskgifts: _Optional[int] = ..., roomFanTicketCount: _Optional[int] = ..., priority: _Optional[_Union[GiftIMPriority, _Mapping]] = ..., gift: _Optional[_Union[GiftStruct, _Mapping]] = ..., logId: _Optional[str] = ..., sendType: _Optional[int] = ..., publicAreaCommon: _Optional[_Union[PublicAreaCommon, _Mapping]] = ..., trayDisplayText: _Optional[_Union[Text, _Mapping]] = ..., bannedDisplayEffects: _Optional[int] = ..., displayForSelf: bool = ..., interactGiftInfo: _Optional[str] = ..., diyItemInfo: _Optional[str] = ..., minAssetSetList: _Optional[_Iterable[int]] = ..., totalCount: _Optional[int] = ..., clientGiftSource: _Optional[int] = ..., toUserIdsList: _Optional[_Iterable[int]] = ..., sendTime: _Optional[int] = ..., forceDisplayEffects: _Optional[int] = ..., traceId: _Optional[str] = ..., effectDisplayTs: _Optional[int] = ...) -> None: ...

class GiftStruct(_message.Message):
    __slots__ = ["image", "describe", "notify", "duration", "id", "forLinkmic", "doodle", "forFansclub", "combo", "type", "diamondCount", "isDisplayedOnPanel", "primaryEffectId", "giftLabelIcon", "name", "region", "manual", "forCustom", "icon", "actionType"]
    IMAGE_FIELD_NUMBER: _ClassVar[int]
    DESCRIBE_FIELD_NUMBER: _ClassVar[int]
    NOTIFY_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    ID_FIELD_NUMBER: _ClassVar[int]
    FORLINKMIC_FIELD_NUMBER: _ClassVar[int]
    DOODLE_FIELD_NUMBER: _ClassVar[int]
    FORFANSCLUB_FIELD_NUMBER: _ClassVar[int]
    COMBO_FIELD_NUMBER: _ClassVar[int]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    DIAMONDCOUNT_FIELD_NUMBER: _ClassVar[int]
    ISDISPLAYEDONPANEL_FIELD_NUMBER: _ClassVar[int]
    PRIMARYEFFECTID_FIELD_NUMBER: _ClassVar[int]
    GIFTLABELICON_FIELD_NUMBER: _ClassVar[int]
    NAME_FIELD_NUMBER: _ClassVar[int]
    REGION_FIELD_NUMBER: _ClassVar[int]
    MANUAL_FIELD_NUMBER: _ClassVar[int]
    FORCUSTOM_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    ACTIONTYPE_FIELD_NUMBER: _ClassVar[int]
    image: Image
    describe: str
    notify: bool
    duration: int
    id: int
    forLinkmic: bool
    doodle: bool
    forFansclub: bool
    combo: bool
    type: int
    diamondCount: int
    isDisplayedOnPanel: bool
    primaryEffectId: int
    giftLabelIcon: Image
    name: str
    region: str
    manual: str
    forCustom: bool
    icon: Image
    actionType: int
    def __init__(self, image: _Optional[_Union[Image, _Mapping]] = ..., describe: _Optional[str] = ..., notify: bool = ..., duration: _Optional[int] = ..., id: _Optional[int] = ..., forLinkmic: bool = ..., doodle: bool = ..., forFansclub: bool = ..., combo: bool = ..., type: _Optional[int] = ..., diamondCount: _Optional[int] = ..., isDisplayedOnPanel: bool = ..., primaryEffectId: _Optional[int] = ..., giftLabelIcon: _Optional[_Union[Image, _Mapping]] = ..., name: _Optional[str] = ..., region: _Optional[str] = ..., manual: _Optional[str] = ..., forCustom: bool = ..., icon: _Optional[_Union[Image, _Mapping]] = ..., actionType: _Optional[int] = ...) -> None: ...

class GiftIMPriority(_message.Message):
    __slots__ = ["queueSizesList", "selfQueuePriority", "priority"]
    QUEUESIZESLIST_FIELD_NUMBER: _ClassVar[int]
    SELFQUEUEPRIORITY_FIELD_NUMBER: _ClassVar[int]
    PRIORITY_FIELD_NUMBER: _ClassVar[int]
    queueSizesList: _containers.RepeatedScalarFieldContainer[int]
    selfQueuePriority: int
    priority: int
    def __init__(self, queueSizesList: _Optional[_Iterable[int]] = ..., selfQueuePriority: _Optional[int] = ..., priority: _Optional[int] = ...) -> None: ...

class TextEffect(_message.Message):
    __slots__ = ["portrait", "landscape"]
    PORTRAIT_FIELD_NUMBER: _ClassVar[int]
    LANDSCAPE_FIELD_NUMBER: _ClassVar[int]
    portrait: TextEffectDetail
    landscape: TextEffectDetail
    def __init__(self, portrait: _Optional[_Union[TextEffectDetail, _Mapping]] = ..., landscape: _Optional[_Union[TextEffectDetail, _Mapping]] = ...) -> None: ...

class TextEffectDetail(_message.Message):
    __slots__ = ["text", "textFontSize", "background", "start", "duration", "x", "y", "width", "height", "shadowDx", "shadowDy", "shadowRadius", "shadowColor", "strokeColor", "strokeWidth"]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    TEXTFONTSIZE_FIELD_NUMBER: _ClassVar[int]
    BACKGROUND_FIELD_NUMBER: _ClassVar[int]
    START_FIELD_NUMBER: _ClassVar[int]
    DURATION_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    SHADOWDX_FIELD_NUMBER: _ClassVar[int]
    SHADOWDY_FIELD_NUMBER: _ClassVar[int]
    SHADOWRADIUS_FIELD_NUMBER: _ClassVar[int]
    SHADOWCOLOR_FIELD_NUMBER: _ClassVar[int]
    STROKECOLOR_FIELD_NUMBER: _ClassVar[int]
    STROKEWIDTH_FIELD_NUMBER: _ClassVar[int]
    text: Text
    textFontSize: int
    background: Image
    start: int
    duration: int
    x: int
    y: int
    width: int
    height: int
    shadowDx: int
    shadowDy: int
    shadowRadius: int
    shadowColor: str
    strokeColor: str
    strokeWidth: int
    def __init__(self, text: _Optional[_Union[Text, _Mapping]] = ..., textFontSize: _Optional[int] = ..., background: _Optional[_Union[Image, _Mapping]] = ..., start: _Optional[int] = ..., duration: _Optional[int] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., width: _Optional[int] = ..., height: _Optional[int] = ..., shadowDx: _Optional[int] = ..., shadowDy: _Optional[int] = ..., shadowRadius: _Optional[int] = ..., shadowColor: _Optional[str] = ..., strokeColor: _Optional[str] = ..., strokeWidth: _Optional[int] = ...) -> None: ...

class MemberMessage(_message.Message):
    __slots__ = ["common", "user", "memberCount", "operator", "isSetToAdmin", "isTopUser", "rankScore", "topUserNo", "enterType", "action", "actionDescription", "userId", "effectConfig", "popStr", "enterEffectConfig", "backgroundImage", "backgroundImageV2", "anchorDisplayText", "publicAreaCommon", "userEnterTipType", "anchorEnterTipType"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    MEMBERCOUNT_FIELD_NUMBER: _ClassVar[int]
    OPERATOR_FIELD_NUMBER: _ClassVar[int]
    ISSETTOADMIN_FIELD_NUMBER: _ClassVar[int]
    ISTOPUSER_FIELD_NUMBER: _ClassVar[int]
    RANKSCORE_FIELD_NUMBER: _ClassVar[int]
    TOPUSERNO_FIELD_NUMBER: _ClassVar[int]
    ENTERTYPE_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    ACTIONDESCRIPTION_FIELD_NUMBER: _ClassVar[int]
    USERID_FIELD_NUMBER: _ClassVar[int]
    EFFECTCONFIG_FIELD_NUMBER: _ClassVar[int]
    POPSTR_FIELD_NUMBER: _ClassVar[int]
    ENTEREFFECTCONFIG_FIELD_NUMBER: _ClassVar[int]
    BACKGROUNDIMAGE_FIELD_NUMBER: _ClassVar[int]
    BACKGROUNDIMAGEV2_FIELD_NUMBER: _ClassVar[int]
    ANCHORDISPLAYTEXT_FIELD_NUMBER: _ClassVar[int]
    PUBLICAREACOMMON_FIELD_NUMBER: _ClassVar[int]
    USERENTERTIPTYPE_FIELD_NUMBER: _ClassVar[int]
    ANCHORENTERTIPTYPE_FIELD_NUMBER: _ClassVar[int]
    common: Common
    user: User
    memberCount: int
    operator: User
    isSetToAdmin: bool
    isTopUser: bool
    rankScore: int
    topUserNo: int
    enterType: int
    action: int
    actionDescription: str
    userId: int
    effectConfig: EffectConfig
    popStr: str
    enterEffectConfig: EffectConfig
    backgroundImage: Image
    backgroundImageV2: Image
    anchorDisplayText: Text
    publicAreaCommon: PublicAreaCommon
    userEnterTipType: int
    anchorEnterTipType: int
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., user: _Optional[_Union[User, _Mapping]] = ..., memberCount: _Optional[int] = ..., operator: _Optional[_Union[User, _Mapping]] = ..., isSetToAdmin: bool = ..., isTopUser: bool = ..., rankScore: _Optional[int] = ..., topUserNo: _Optional[int] = ..., enterType: _Optional[int] = ..., action: _Optional[int] = ..., actionDescription: _Optional[str] = ..., userId: _Optional[int] = ..., effectConfig: _Optional[_Union[EffectConfig, _Mapping]] = ..., popStr: _Optional[str] = ..., enterEffectConfig: _Optional[_Union[EffectConfig, _Mapping]] = ..., backgroundImage: _Optional[_Union[Image, _Mapping]] = ..., backgroundImageV2: _Optional[_Union[Image, _Mapping]] = ..., anchorDisplayText: _Optional[_Union[Text, _Mapping]] = ..., publicAreaCommon: _Optional[_Union[PublicAreaCommon, _Mapping]] = ..., userEnterTipType: _Optional[int] = ..., anchorEnterTipType: _Optional[int] = ...) -> None: ...

class PublicAreaCommon(_message.Message):
    __slots__ = ["userLabel", "userConsumeInRoom", "userSendGiftCntInRoom"]
    USERLABEL_FIELD_NUMBER: _ClassVar[int]
    USERCONSUMEINROOM_FIELD_NUMBER: _ClassVar[int]
    USERSENDGIFTCNTINROOM_FIELD_NUMBER: _ClassVar[int]
    userLabel: Image
    userConsumeInRoom: int
    userSendGiftCntInRoom: int
    def __init__(self, userLabel: _Optional[_Union[Image, _Mapping]] = ..., userConsumeInRoom: _Optional[int] = ..., userSendGiftCntInRoom: _Optional[int] = ...) -> None: ...

class EffectConfig(_message.Message):
    __slots__ = ["type", "icon", "avatarPos", "text", "textIcon", "stayTime", "animAssetId", "badge", "flexSettingArrayList", "textIconOverlay", "animatedBadge", "hasSweepLight", "textFlexSettingArrayList", "centerAnimAssetId", "dynamicImage", "extraMap", "mp4AnimAssetId", "priority", "maxWaitTime", "dressId", "alignment", "alignmentOffset"]
    class ExtraMapEntry(_message.Message):
        __slots__ = ["key", "value"]
        KEY_FIELD_NUMBER: _ClassVar[int]
        VALUE_FIELD_NUMBER: _ClassVar[int]
        key: str
        value: str
        def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
    TYPE_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    AVATARPOS_FIELD_NUMBER: _ClassVar[int]
    TEXT_FIELD_NUMBER: _ClassVar[int]
    TEXTICON_FIELD_NUMBER: _ClassVar[int]
    STAYTIME_FIELD_NUMBER: _ClassVar[int]
    ANIMASSETID_FIELD_NUMBER: _ClassVar[int]
    BADGE_FIELD_NUMBER: _ClassVar[int]
    FLEXSETTINGARRAYLIST_FIELD_NUMBER: _ClassVar[int]
    TEXTICONOVERLAY_FIELD_NUMBER: _ClassVar[int]
    ANIMATEDBADGE_FIELD_NUMBER: _ClassVar[int]
    HASSWEEPLIGHT_FIELD_NUMBER: _ClassVar[int]
    TEXTFLEXSETTINGARRAYLIST_FIELD_NUMBER: _ClassVar[int]
    CENTERANIMASSETID_FIELD_NUMBER: _ClassVar[int]
    DYNAMICIMAGE_FIELD_NUMBER: _ClassVar[int]
    EXTRAMAP_FIELD_NUMBER: _ClassVar[int]
    MP4ANIMASSETID_FIELD_NUMBER: _ClassVar[int]
    PRIORITY_FIELD_NUMBER: _ClassVar[int]
    MAXWAITTIME_FIELD_NUMBER: _ClassVar[int]
    DRESSID_FIELD_NUMBER: _ClassVar[int]
    ALIGNMENT_FIELD_NUMBER: _ClassVar[int]
    ALIGNMENTOFFSET_FIELD_NUMBER: _ClassVar[int]
    type: int
    icon: Image
    avatarPos: int
    text: Text
    textIcon: Image
    stayTime: int
    animAssetId: int
    badge: Image
    flexSettingArrayList: _containers.RepeatedScalarFieldContainer[int]
    textIconOverlay: Image
    animatedBadge: Image
    hasSweepLight: bool
    textFlexSettingArrayList: _containers.RepeatedScalarFieldContainer[int]
    centerAnimAssetId: int
    dynamicImage: Image
    extraMap: _containers.ScalarMap[str, str]
    mp4AnimAssetId: int
    priority: int
    maxWaitTime: int
    dressId: str
    alignment: int
    alignmentOffset: int
    def __init__(self, type: _Optional[int] = ..., icon: _Optional[_Union[Image, _Mapping]] = ..., avatarPos: _Optional[int] = ..., text: _Optional[_Union[Text, _Mapping]] = ..., textIcon: _Optional[_Union[Image, _Mapping]] = ..., stayTime: _Optional[int] = ..., animAssetId: _Optional[int] = ..., badge: _Optional[_Union[Image, _Mapping]] = ..., flexSettingArrayList: _Optional[_Iterable[int]] = ..., textIconOverlay: _Optional[_Union[Image, _Mapping]] = ..., animatedBadge: _Optional[_Union[Image, _Mapping]] = ..., hasSweepLight: bool = ..., textFlexSettingArrayList: _Optional[_Iterable[int]] = ..., centerAnimAssetId: _Optional[int] = ..., dynamicImage: _Optional[_Union[Image, _Mapping]] = ..., extraMap: _Optional[_Mapping[str, str]] = ..., mp4AnimAssetId: _Optional[int] = ..., priority: _Optional[int] = ..., maxWaitTime: _Optional[int] = ..., dressId: _Optional[str] = ..., alignment: _Optional[int] = ..., alignmentOffset: _Optional[int] = ...) -> None: ...

class Text(_message.Message):
    __slots__ = ["key", "defaultPatter", "defaultFormat", "piecesList"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    DEFAULTPATTER_FIELD_NUMBER: _ClassVar[int]
    DEFAULTFORMAT_FIELD_NUMBER: _ClassVar[int]
    PIECESLIST_FIELD_NUMBER: _ClassVar[int]
    key: str
    defaultPatter: str
    defaultFormat: TextFormat
    piecesList: _containers.RepeatedCompositeFieldContainer[TextPiece]
    def __init__(self, key: _Optional[str] = ..., defaultPatter: _Optional[str] = ..., defaultFormat: _Optional[_Union[TextFormat, _Mapping]] = ..., piecesList: _Optional[_Iterable[_Union[TextPiece, _Mapping]]] = ...) -> None: ...

class TextPiece(_message.Message):
    __slots__ = ["type", "format", "stringValue", "userValue", "giftValue", "heartValue", "patternRefValue", "imageValue"]
    TYPE_FIELD_NUMBER: _ClassVar[int]
    FORMAT_FIELD_NUMBER: _ClassVar[int]
    STRINGVALUE_FIELD_NUMBER: _ClassVar[int]
    USERVALUE_FIELD_NUMBER: _ClassVar[int]
    GIFTVALUE_FIELD_NUMBER: _ClassVar[int]
    HEARTVALUE_FIELD_NUMBER: _ClassVar[int]
    PATTERNREFVALUE_FIELD_NUMBER: _ClassVar[int]
    IMAGEVALUE_FIELD_NUMBER: _ClassVar[int]
    type: bool
    format: TextFormat
    stringValue: str
    userValue: TextPieceUser
    giftValue: TextPieceGift
    heartValue: TextPieceHeart
    patternRefValue: TextPiecePatternRef
    imageValue: TextPieceImage
    def __init__(self, type: bool = ..., format: _Optional[_Union[TextFormat, _Mapping]] = ..., stringValue: _Optional[str] = ..., userValue: _Optional[_Union[TextPieceUser, _Mapping]] = ..., giftValue: _Optional[_Union[TextPieceGift, _Mapping]] = ..., heartValue: _Optional[_Union[TextPieceHeart, _Mapping]] = ..., patternRefValue: _Optional[_Union[TextPiecePatternRef, _Mapping]] = ..., imageValue: _Optional[_Union[TextPieceImage, _Mapping]] = ...) -> None: ...

class TextPieceImage(_message.Message):
    __slots__ = ["image", "scalingRate"]
    IMAGE_FIELD_NUMBER: _ClassVar[int]
    SCALINGRATE_FIELD_NUMBER: _ClassVar[int]
    image: Image
    scalingRate: float
    def __init__(self, image: _Optional[_Union[Image, _Mapping]] = ..., scalingRate: _Optional[float] = ...) -> None: ...

class TextPiecePatternRef(_message.Message):
    __slots__ = ["key", "defaultPattern"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    DEFAULTPATTERN_FIELD_NUMBER: _ClassVar[int]
    key: str
    defaultPattern: str
    def __init__(self, key: _Optional[str] = ..., defaultPattern: _Optional[str] = ...) -> None: ...

class TextPieceHeart(_message.Message):
    __slots__ = ["color"]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    color: str
    def __init__(self, color: _Optional[str] = ...) -> None: ...

class TextPieceGift(_message.Message):
    __slots__ = ["giftId", "nameRef"]
    GIFTID_FIELD_NUMBER: _ClassVar[int]
    NAMEREF_FIELD_NUMBER: _ClassVar[int]
    giftId: int
    nameRef: PatternRef
    def __init__(self, giftId: _Optional[int] = ..., nameRef: _Optional[_Union[PatternRef, _Mapping]] = ...) -> None: ...

class PatternRef(_message.Message):
    __slots__ = ["key", "defaultPattern"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    DEFAULTPATTERN_FIELD_NUMBER: _ClassVar[int]
    key: str
    defaultPattern: str
    def __init__(self, key: _Optional[str] = ..., defaultPattern: _Optional[str] = ...) -> None: ...

class TextPieceUser(_message.Message):
    __slots__ = ["user", "withColon"]
    USER_FIELD_NUMBER: _ClassVar[int]
    WITHCOLON_FIELD_NUMBER: _ClassVar[int]
    user: User
    withColon: bool
    def __init__(self, user: _Optional[_Union[User, _Mapping]] = ..., withColon: bool = ...) -> None: ...

class TextFormat(_message.Message):
    __slots__ = ["color", "bold", "italic", "weight", "italicAngle", "fontSize", "useHeighLightColor", "useRemoteClor"]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    BOLD_FIELD_NUMBER: _ClassVar[int]
    ITALIC_FIELD_NUMBER: _ClassVar[int]
    WEIGHT_FIELD_NUMBER: _ClassVar[int]
    ITALICANGLE_FIELD_NUMBER: _ClassVar[int]
    FONTSIZE_FIELD_NUMBER: _ClassVar[int]
    USEHEIGHLIGHTCOLOR_FIELD_NUMBER: _ClassVar[int]
    USEREMOTECLOR_FIELD_NUMBER: _ClassVar[int]
    color: str
    bold: bool
    italic: bool
    weight: int
    italicAngle: int
    fontSize: int
    useHeighLightColor: bool
    useRemoteClor: bool
    def __init__(self, color: _Optional[str] = ..., bold: bool = ..., italic: bool = ..., weight: _Optional[int] = ..., italicAngle: _Optional[int] = ..., fontSize: _Optional[int] = ..., useHeighLightColor: bool = ..., useRemoteClor: bool = ...) -> None: ...

class LikeMessage(_message.Message):
    __slots__ = ["common", "count", "total", "color", "user", "icon", "doubleLikeDetail", "displayControlInfo", "linkmicGuestUid", "scene", "picoDisplayInfo"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    COUNT_FIELD_NUMBER: _ClassVar[int]
    TOTAL_FIELD_NUMBER: _ClassVar[int]
    COLOR_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    ICON_FIELD_NUMBER: _ClassVar[int]
    DOUBLELIKEDETAIL_FIELD_NUMBER: _ClassVar[int]
    DISPLAYCONTROLINFO_FIELD_NUMBER: _ClassVar[int]
    LINKMICGUESTUID_FIELD_NUMBER: _ClassVar[int]
    SCENE_FIELD_NUMBER: _ClassVar[int]
    PICODISPLAYINFO_FIELD_NUMBER: _ClassVar[int]
    common: Common
    count: int
    total: int
    color: int
    user: User
    icon: str
    doubleLikeDetail: DoubleLikeDetail
    displayControlInfo: DisplayControlInfo
    linkmicGuestUid: int
    scene: str
    picoDisplayInfo: PicoDisplayInfo
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., count: _Optional[int] = ..., total: _Optional[int] = ..., color: _Optional[int] = ..., user: _Optional[_Union[User, _Mapping]] = ..., icon: _Optional[str] = ..., doubleLikeDetail: _Optional[_Union[DoubleLikeDetail, _Mapping]] = ..., displayControlInfo: _Optional[_Union[DisplayControlInfo, _Mapping]] = ..., linkmicGuestUid: _Optional[int] = ..., scene: _Optional[str] = ..., picoDisplayInfo: _Optional[_Union[PicoDisplayInfo, _Mapping]] = ...) -> None: ...

class SocialMessage(_message.Message):
    __slots__ = ["common", "user", "shareType", "action", "shareTarget", "followCount", "publicAreaCommon"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    SHARETYPE_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    SHARETARGET_FIELD_NUMBER: _ClassVar[int]
    FOLLOWCOUNT_FIELD_NUMBER: _ClassVar[int]
    PUBLICAREACOMMON_FIELD_NUMBER: _ClassVar[int]
    common: Common
    user: User
    shareType: int
    action: int
    shareTarget: str
    followCount: int
    publicAreaCommon: PublicAreaCommon
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., user: _Optional[_Union[User, _Mapping]] = ..., shareType: _Optional[int] = ..., action: _Optional[int] = ..., shareTarget: _Optional[str] = ..., followCount: _Optional[int] = ..., publicAreaCommon: _Optional[_Union[PublicAreaCommon, _Mapping]] = ...) -> None: ...

class PicoDisplayInfo(_message.Message):
    __slots__ = ["comboSumCount", "emoji", "emojiIcon", "emojiText"]
    COMBOSUMCOUNT_FIELD_NUMBER: _ClassVar[int]
    EMOJI_FIELD_NUMBER: _ClassVar[int]
    EMOJIICON_FIELD_NUMBER: _ClassVar[int]
    EMOJITEXT_FIELD_NUMBER: _ClassVar[int]
    comboSumCount: int
    emoji: str
    emojiIcon: Image
    emojiText: str
    def __init__(self, comboSumCount: _Optional[int] = ..., emoji: _Optional[str] = ..., emojiIcon: _Optional[_Union[Image, _Mapping]] = ..., emojiText: _Optional[str] = ...) -> None: ...

class DoubleLikeDetail(_message.Message):
    __slots__ = ["doubleFlag", "seqId", "renewalsNum", "triggersNum"]
    DOUBLEFLAG_FIELD_NUMBER: _ClassVar[int]
    SEQID_FIELD_NUMBER: _ClassVar[int]
    RENEWALSNUM_FIELD_NUMBER: _ClassVar[int]
    TRIGGERSNUM_FIELD_NUMBER: _ClassVar[int]
    doubleFlag: bool
    seqId: int
    renewalsNum: int
    triggersNum: int
    def __init__(self, doubleFlag: bool = ..., seqId: _Optional[int] = ..., renewalsNum: _Optional[int] = ..., triggersNum: _Optional[int] = ...) -> None: ...

class DisplayControlInfo(_message.Message):
    __slots__ = ["showText", "showIcons"]
    SHOWTEXT_FIELD_NUMBER: _ClassVar[int]
    SHOWICONS_FIELD_NUMBER: _ClassVar[int]
    showText: bool
    showIcons: bool
    def __init__(self, showText: bool = ..., showIcons: bool = ...) -> None: ...

class EpisodeChatMessage(_message.Message):
    __slots__ = ["common", "user", "content", "visibleToSende", "giftImage", "agreeMsgId", "colorValueList"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    VISIBLETOSENDE_FIELD_NUMBER: _ClassVar[int]
    GIFTIMAGE_FIELD_NUMBER: _ClassVar[int]
    AGREEMSGID_FIELD_NUMBER: _ClassVar[int]
    COLORVALUELIST_FIELD_NUMBER: _ClassVar[int]
    common: Message
    user: User
    content: str
    visibleToSende: bool
    giftImage: Image
    agreeMsgId: int
    colorValueList: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, common: _Optional[_Union[Message, _Mapping]] = ..., user: _Optional[_Union[User, _Mapping]] = ..., content: _Optional[str] = ..., visibleToSende: bool = ..., giftImage: _Optional[_Union[Image, _Mapping]] = ..., agreeMsgId: _Optional[int] = ..., colorValueList: _Optional[_Iterable[str]] = ...) -> None: ...

class MatchAgainstScoreMessage(_message.Message):
    __slots__ = ["common", "against", "matchStatus", "displayStatus"]
    COMMON_FIELD_NUMBER: _ClassVar[int]
    AGAINST_FIELD_NUMBER: _ClassVar[int]
    MATCHSTATUS_FIELD_NUMBER: _ClassVar[int]
    DISPLAYSTATUS_FIELD_NUMBER: _ClassVar[int]
    common: Common
    against: Against
    matchStatus: int
    displayStatus: int
    def __init__(self, common: _Optional[_Union[Common, _Mapping]] = ..., against: _Optional[_Union[Against, _Mapping]] = ..., matchStatus: _Optional[int] = ..., displayStatus: _Optional[int] = ...) -> None: ...

class Against(_message.Message):
    __slots__ = ["leftName", "leftLogo", "leftGoal", "rightName", "rightLogo", "rightGoal", "timestamp", "version", "leftTeamId", "rightTeamId", "diffSei2absSecond", "finalGoalStage", "currentGoalStage", "leftScoreAddition", "rightScoreAddition", "leftGoalInt", "rightGoalInt"]
    LEFTNAME_FIELD_NUMBER: _ClassVar[int]
    LEFTLOGO_FIELD_NUMBER: _ClassVar[int]
    LEFTGOAL_FIELD_NUMBER: _ClassVar[int]
    RIGHTNAME_FIELD_NUMBER: _ClassVar[int]
    RIGHTLOGO_FIELD_NUMBER: _ClassVar[int]
    RIGHTGOAL_FIELD_NUMBER: _ClassVar[int]
    TIMESTAMP_FIELD_NUMBER: _ClassVar[int]
    VERSION_FIELD_NUMBER: _ClassVar[int]
    LEFTTEAMID_FIELD_NUMBER: _ClassVar[int]
    RIGHTTEAMID_FIELD_NUMBER: _ClassVar[int]
    DIFFSEI2ABSSECOND_FIELD_NUMBER: _ClassVar[int]
    FINALGOALSTAGE_FIELD_NUMBER: _ClassVar[int]
    CURRENTGOALSTAGE_FIELD_NUMBER: _ClassVar[int]
    LEFTSCOREADDITION_FIELD_NUMBER: _ClassVar[int]
    RIGHTSCOREADDITION_FIELD_NUMBER: _ClassVar[int]
    LEFTGOALINT_FIELD_NUMBER: _ClassVar[int]
    RIGHTGOALINT_FIELD_NUMBER: _ClassVar[int]
    leftName: str
    leftLogo: Image
    leftGoal: str
    rightName: str
    rightLogo: Image
    rightGoal: str
    timestamp: int
    version: int
    leftTeamId: int
    rightTeamId: int
    diffSei2absSecond: int
    finalGoalStage: int
    currentGoalStage: int
    leftScoreAddition: int
    rightScoreAddition: int
    leftGoalInt: int
    rightGoalInt: int
    def __init__(self, leftName: _Optional[str] = ..., leftLogo: _Optional[_Union[Image, _Mapping]] = ..., leftGoal: _Optional[str] = ..., rightName: _Optional[str] = ..., rightLogo: _Optional[_Union[Image, _Mapping]] = ..., rightGoal: _Optional[str] = ..., timestamp: _Optional[int] = ..., version: _Optional[int] = ..., leftTeamId: _Optional[int] = ..., rightTeamId: _Optional[int] = ..., diffSei2absSecond: _Optional[int] = ..., finalGoalStage: _Optional[int] = ..., currentGoalStage: _Optional[int] = ..., leftScoreAddition: _Optional[int] = ..., rightScoreAddition: _Optional[int] = ..., leftGoalInt: _Optional[int] = ..., rightGoalInt: _Optional[int] = ...) -> None: ...

class Common(_message.Message):
    __slots__ = ["method", "msgId", "roomId", "createTime", "monitor", "isShowMsg", "describe", "foldType", "anchorFoldType", "priorityScore", "logId", "msgProcessFilterK", "msgProcessFilterV", "user", "anchorFoldTypeV2", "processAtSeiTimeMs", "randomDispatchMs", "isDispatch", "channelId", "diffSei2absSecond", "anchorFoldDuration"]
    METHOD_FIELD_NUMBER: _ClassVar[int]
    MSGID_FIELD_NUMBER: _ClassVar[int]
    ROOMID_FIELD_NUMBER: _ClassVar[int]
    CREATETIME_FIELD_NUMBER: _ClassVar[int]
    MONITOR_FIELD_NUMBER: _ClassVar[int]
    ISSHOWMSG_FIELD_NUMBER: _ClassVar[int]
    DESCRIBE_FIELD_NUMBER: _ClassVar[int]
    FOLDTYPE_FIELD_NUMBER: _ClassVar[int]
    ANCHORFOLDTYPE_FIELD_NUMBER: _ClassVar[int]
    PRIORITYSCORE_FIELD_NUMBER: _ClassVar[int]
    LOGID_FIELD_NUMBER: _ClassVar[int]
    MSGPROCESSFILTERK_FIELD_NUMBER: _ClassVar[int]
    MSGPROCESSFILTERV_FIELD_NUMBER: _ClassVar[int]
    USER_FIELD_NUMBER: _ClassVar[int]
    ANCHORFOLDTYPEV2_FIELD_NUMBER: _ClassVar[int]
    PROCESSATSEITIMEMS_FIELD_NUMBER: _ClassVar[int]
    RANDOMDISPATCHMS_FIELD_NUMBER: _ClassVar[int]
    ISDISPATCH_FIELD_NUMBER: _ClassVar[int]
    CHANNELID_FIELD_NUMBER: _ClassVar[int]
    DIFFSEI2ABSSECOND_FIELD_NUMBER: _ClassVar[int]
    ANCHORFOLDDURATION_FIELD_NUMBER: _ClassVar[int]
    method: str
    msgId: int
    roomId: int
    createTime: int
    monitor: int
    isShowMsg: bool
    describe: str
    foldType: int
    anchorFoldType: int
    priorityScore: int
    logId: str
    msgProcessFilterK: str
    msgProcessFilterV: str
    user: User
    anchorFoldTypeV2: int
    processAtSeiTimeMs: int
    randomDispatchMs: int
    isDispatch: bool
    channelId: int
    diffSei2absSecond: int
    anchorFoldDuration: int
    def __init__(self, method: _Optional[str] = ..., msgId: _Optional[int] = ..., roomId: _Optional[int] = ..., createTime: _Optional[int] = ..., monitor: _Optional[int] = ..., isShowMsg: bool = ..., describe: _Optional[str] = ..., foldType: _Optional[int] = ..., anchorFoldType: _Optional[int] = ..., priorityScore: _Optional[int] = ..., logId: _Optional[str] = ..., msgProcessFilterK: _Optional[str] = ..., msgProcessFilterV: _Optional[str] = ..., user: _Optional[_Union[User, _Mapping]] = ..., anchorFoldTypeV2: _Optional[int] = ..., processAtSeiTimeMs: _Optional[int] = ..., randomDispatchMs: _Optional[int] = ..., isDispatch: bool = ..., channelId: _Optional[int] = ..., diffSei2absSecond: _Optional[int] = ..., anchorFoldDuration: _Optional[int] = ...) -> None: ...

class User(_message.Message):
    __slots__ = ["id", "shortId", "nickName", "gender", "Signature", "Level", "Birthday", "Telephone", "AvatarThumb", "AvatarMedium", "AvatarLarge", "Verified", "Experience", "city", "Status", "CreateTime", "ModifyTime", "Secret", "ShareQrcodeUri", "IncomeSharePercent", "BadgeImageList", "SpecialId", "AvatarBorder", "Medal", "RealTimeIconsList"]
    ID_FIELD_NUMBER: _ClassVar[int]
    SHORTID_FIELD_NUMBER: _ClassVar[int]
    NICKNAME_FIELD_NUMBER: _ClassVar[int]
    GENDER_FIELD_NUMBER: _ClassVar[int]
    SIGNATURE_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    BIRTHDAY_FIELD_NUMBER: _ClassVar[int]
    TELEPHONE_FIELD_NUMBER: _ClassVar[int]
    AVATARTHUMB_FIELD_NUMBER: _ClassVar[int]
    AVATARMEDIUM_FIELD_NUMBER: _ClassVar[int]
    AVATARLARGE_FIELD_NUMBER: _ClassVar[int]
    VERIFIED_FIELD_NUMBER: _ClassVar[int]
    EXPERIENCE_FIELD_NUMBER: _ClassVar[int]
    CITY_FIELD_NUMBER: _ClassVar[int]
    STATUS_FIELD_NUMBER: _ClassVar[int]
    CREATETIME_FIELD_NUMBER: _ClassVar[int]
    MODIFYTIME_FIELD_NUMBER: _ClassVar[int]
    SECRET_FIELD_NUMBER: _ClassVar[int]
    SHAREQRCODEURI_FIELD_NUMBER: _ClassVar[int]
    INCOMESHAREPERCENT_FIELD_NUMBER: _ClassVar[int]
    BADGEIMAGELIST_FIELD_NUMBER: _ClassVar[int]
    SPECIALID_FIELD_NUMBER: _ClassVar[int]
    AVATARBORDER_FIELD_NUMBER: _ClassVar[int]
    MEDAL_FIELD_NUMBER: _ClassVar[int]
    REALTIMEICONSLIST_FIELD_NUMBER: _ClassVar[int]
    id: int
    shortId: int
    nickName: str
    gender: int
    Signature: str
    Level: int
    Birthday: int
    Telephone: str
    AvatarThumb: Image
    AvatarMedium: Image
    AvatarLarge: Image
    Verified: bool
    Experience: int
    city: str
    Status: int
    CreateTime: int
    ModifyTime: int
    Secret: int
    ShareQrcodeUri: str
    IncomeSharePercent: int
    BadgeImageList: _containers.RepeatedCompositeFieldContainer[Image]
    SpecialId: str
    AvatarBorder: Image
    Medal: Image
    RealTimeIconsList: _containers.RepeatedCompositeFieldContainer[Image]
    def __init__(self, id: _Optional[int] = ..., shortId: _Optional[int] = ..., nickName: _Optional[str] = ..., gender: _Optional[int] = ..., Signature: _Optional[str] = ..., Level: _Optional[int] = ..., Birthday: _Optional[int] = ..., Telephone: _Optional[str] = ..., AvatarThumb: _Optional[_Union[Image, _Mapping]] = ..., AvatarMedium: _Optional[_Union[Image, _Mapping]] = ..., AvatarLarge: _Optional[_Union[Image, _Mapping]] = ..., Verified: bool = ..., Experience: _Optional[int] = ..., city: _Optional[str] = ..., Status: _Optional[int] = ..., CreateTime: _Optional[int] = ..., ModifyTime: _Optional[int] = ..., Secret: _Optional[int] = ..., ShareQrcodeUri: _Optional[str] = ..., IncomeSharePercent: _Optional[int] = ..., BadgeImageList: _Optional[_Iterable[_Union[Image, _Mapping]]] = ..., SpecialId: _Optional[str] = ..., AvatarBorder: _Optional[_Union[Image, _Mapping]] = ..., Medal: _Optional[_Union[Image, _Mapping]] = ..., RealTimeIconsList: _Optional[_Iterable[_Union[Image, _Mapping]]] = ...) -> None: ...

class Image(_message.Message):
    __slots__ = ["urlListList", "uri", "height", "width", "avgColor", "imageType", "openWebUrl", "content", "isAnimated", "FlexSettingList", "TextSettingList"]
    URLLISTLIST_FIELD_NUMBER: _ClassVar[int]
    URI_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    AVGCOLOR_FIELD_NUMBER: _ClassVar[int]
    IMAGETYPE_FIELD_NUMBER: _ClassVar[int]
    OPENWEBURL_FIELD_NUMBER: _ClassVar[int]
    CONTENT_FIELD_NUMBER: _ClassVar[int]
    ISANIMATED_FIELD_NUMBER: _ClassVar[int]
    FLEXSETTINGLIST_FIELD_NUMBER: _ClassVar[int]
    TEXTSETTINGLIST_FIELD_NUMBER: _ClassVar[int]
    urlListList: _containers.RepeatedScalarFieldContainer[str]
    uri: str
    height: int
    width: int
    avgColor: str
    imageType: int
    openWebUrl: str
    content: ImageContent
    isAnimated: bool
    FlexSettingList: NinePatchSetting
    TextSettingList: NinePatchSetting
    def __init__(self, urlListList: _Optional[_Iterable[str]] = ..., uri: _Optional[str] = ..., height: _Optional[int] = ..., width: _Optional[int] = ..., avgColor: _Optional[str] = ..., imageType: _Optional[int] = ..., openWebUrl: _Optional[str] = ..., content: _Optional[_Union[ImageContent, _Mapping]] = ..., isAnimated: bool = ..., FlexSettingList: _Optional[_Union[NinePatchSetting, _Mapping]] = ..., TextSettingList: _Optional[_Union[NinePatchSetting, _Mapping]] = ...) -> None: ...

class NinePatchSetting(_message.Message):
    __slots__ = ["settingListList"]
    SETTINGLISTLIST_FIELD_NUMBER: _ClassVar[int]
    settingListList: _containers.RepeatedScalarFieldContainer[str]
    def __init__(self, settingListList: _Optional[_Iterable[str]] = ...) -> None: ...

class ImageContent(_message.Message):
    __slots__ = ["name", "fontColor", "level", "alternativeText"]
    NAME_FIELD_NUMBER: _ClassVar[int]
    FONTCOLOR_FIELD_NUMBER: _ClassVar[int]
    LEVEL_FIELD_NUMBER: _ClassVar[int]
    ALTERNATIVETEXT_FIELD_NUMBER: _ClassVar[int]
    name: str
    fontColor: str
    level: int
    alternativeText: str
    def __init__(self, name: _Optional[str] = ..., fontColor: _Optional[str] = ..., level: _Optional[int] = ..., alternativeText: _Optional[str] = ...) -> None: ...

class PushFrame(_message.Message):
    __slots__ = ["seqId", "logId", "service", "method", "headersList", "payloadEncoding", "payloadType", "payload"]
    SEQID_FIELD_NUMBER: _ClassVar[int]
    LOGID_FIELD_NUMBER: _ClassVar[int]
    SERVICE_FIELD_NUMBER: _ClassVar[int]
    METHOD_FIELD_NUMBER: _ClassVar[int]
    HEADERSLIST_FIELD_NUMBER: _ClassVar[int]
    PAYLOADENCODING_FIELD_NUMBER: _ClassVar[int]
    PAYLOADTYPE_FIELD_NUMBER: _ClassVar[int]
    PAYLOAD_FIELD_NUMBER: _ClassVar[int]
    seqId: int
    logId: int
    service: int
    method: int
    headersList: _containers.RepeatedCompositeFieldContainer[HeadersList]
    payloadEncoding: str
    payloadType: str
    payload: bytes
    def __init__(self, seqId: _Optional[int] = ..., logId: _Optional[int] = ..., service: _Optional[int] = ..., method: _Optional[int] = ..., headersList: _Optional[_Iterable[_Union[HeadersList, _Mapping]]] = ..., payloadEncoding: _Optional[str] = ..., payloadType: _Optional[str] = ..., payload: _Optional[bytes] = ...) -> None: ...

class HeadersList(_message.Message):
    __slots__ = ["key", "value"]
    KEY_FIELD_NUMBER: _ClassVar[int]
    VALUE_FIELD_NUMBER: _ClassVar[int]
    key: str
    value: str
    def __init__(self, key: _Optional[str] = ..., value: _Optional[str] = ...) -> None: ...
