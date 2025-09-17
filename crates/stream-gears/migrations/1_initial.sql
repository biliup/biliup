create table if not exists uploadstreamers
(
    id INTEGER not null
        constraint pk_uploadstreamers
            primary key,
    template_name VARCHAR not null,
    title VARCHAR,
    tid INTEGER,
    copyright INTEGER,
    copyright_source VARCHAR,
    cover_path VARCHAR,
    description TEXT,
    dynamic VARCHAR,
    dtime INTEGER,
    dolby INTEGER,
    hires INTEGER,
    charging_pay INTEGER,
    no_reprint INTEGER,
    uploader VARCHAR,
    user_cookie VARCHAR,
    tags JSON not null,
    credits JSON,
    up_selection_reply INTEGER,
    up_close_reply INTEGER,
    up_close_danmu INTEGER,
    extra_fields VARCHAR,
    is_only_self INTEGER
);
create table if not exists streamerinfo
(
    id INTEGER not null
        constraint pk_streamerinfo
            primary key,
    name VARCHAR not null,
    url VARCHAR not null,
    title VARCHAR not null,
    date DATETIME not null,
    live_cover_path VARCHAR not null
);
create table if not exists livestreamers
(
    id INTEGER not null
        constraint pk_livestreamers
            primary key,
    url VARCHAR not null
        constraint uq_livestreamers_url
            unique,
    remark VARCHAR not null,
    filename_prefix VARCHAR,
    time_range VARCHAR,
    upload_streamers_id INTEGER
        constraint fk_livestreamers_upload_streamers_id_uploadstreamers
            references uploadstreamers
            on delete cascade,
    format VARCHAR,
    override JSON,
    preprocessor JSON,
    segment_processor JSON,
    downloaded_processor JSON,
    postprocessor JSON,
    opt_args JSON,
    excluded_keywords JSON
);
create table if not exists filelist
(
    id INTEGER not null
        constraint pk_filelist
            primary key,
    file VARCHAR not null,
    streamer_info_id INTEGER not null
        constraint fk_filelist_streamer_info_id_streamerinfo
            references streamerinfo
            on delete cascade
);
create table if not exists configuration
(
    id INTEGER not null
        constraint pk_configuration
            primary key,
    key VARCHAR not null,
    value TEXT not null
);


-- create table if not exists live_streamers
-- (
--     id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     url        TEXT                              not null UNIQUE default '',
--     remark     TEXT                              not null        default '',
--     filename   TEXT                              not null        default './video/%Y-%m-%d/%H_%M_%S{title}',
--     split_time INTEGER,
--     split_size INTEGER,
--     upload_id  INTEGER
-- );
--
-- create table if not exists upload_streamers
-- (
--     id                 INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     template_name      TEXT                              NOT NULL UNIQUE,
--     user               INTEGER                           NOT NULL,
--     copyright          INTEGER                           not null default 1,
--     source             TEXT                              not null default '',
--     tid                INTEGER                           not null default 171,
--     cover              TEXT                              not null default '',
--     title              TEXT                              not null default '',
--     'desc'             TEXT                              not null default '',
--     dynamic            TEXT                              not null default '',
--     tag                TEXT                              not null default '',
--     dtime              INTEGER,
--     interactive        INTEGER                           not null default 0,
--     mission_id         INTEGER,
--     dolby              INTEGER                           not null default 0,
--     lossless_music     INTEGER                           not null default 0,
--     no_reprint         INTEGER                           not null default 0,
--     charging_pay          INTEGER                           not null default 0,
--     up_selection_reply INTEGER                           not null default 0,
--     up_close_reply     INTEGER                           not null default 0,
--     up_close_danmu     INTEGER                           not null default 0
-- );
--
-- create table if not exists upload_records
-- (
--     id       INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     identity TEXT                              not null,
--     status   TEXT
-- );
--
--
-- create table if not exists users
-- (
--     id       INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     name     TEXT                              not null UNIQUE,
--     value    TEXT                              not null,
--     platform TEXT                              not null
-- );
--
-- create table if not exists download_records
-- (
--     id        INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     title     TEXT                              not null,
--     live_id   INTEGER                           not null,
--     upload_id INTEGER,
--     status    TEXT
-- );
--
--
-- create table if not exists videos
-- (
--     id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
--     start_time INTEGER                           not null,
--     end_time   INTEGER                           not null,
--     path       TEXT                              not null,
--     record_id  INTEGER                           not null,
--     status     TEXT
-- );
--
--
-- -- alter table users
-- --     add constraint users_id_pk primary key (id);
-- --
-- -- create index if not exists users_email_idx on users (email);
-- -- insert or ignore into upload_streamers (template_name, copyright, source, cover, title, desc, dynamic, tag, dtime,
-- --                                         interactive, mission_id, dolby, lossless_music, no_reprint, charging_pay,
-- --                                         up_selection_reply, up_close_reply, up_close_danmu)
-- -- values ('空模板', 1, '', '', '', '', '', '', null, 0, null, 0, 0, null, 0, 0, 0, null);