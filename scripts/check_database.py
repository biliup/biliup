#!/usr/bin/env python3
"""
数据库配置检查脚本

使用方法:
python scripts/check_database.py
"""

import os
import sys
from pathlib import Path

# 添加项目根目录到 Python 路径
project_root = Path(__file__).parent.parent
sys.path.insert(0, str(project_root))

from biliup.database.models import get_database_url, engine, BaseModel


def check_database_config():
    """检查数据库配置"""
    print("=" * 50)
    print("数据库配置检查")
    print("=" * 50)
    
    # 检查数据库类型
    db_type = os.getenv('BILIUP_DB_TYPE', 'sqlite').lower()
    print(f"数据库类型: {db_type}")
    
    if db_type == 'mysql':
        # 检查MySQL环境变量
        mysql_vars = {
            'BILIUP_MYSQL_HOST': 'MySQL主机',
            'BILIUP_MYSQL_PORT': 'MySQL端口',
            'BILIUP_MYSQL_DATABASE': '数据库名',
            'BILIUP_MYSQL_USERNAME': '用户名',
            'BILIUP_MYSQL_PASSWORD': '密码',
            'BILIUP_MYSQL_CHARSET': '字符集'
        }
        
        print("\nMySQL配置:")
        missing_vars = []
        for var, desc in mysql_vars.items():
            value = os.getenv(var)
            if value:
                if 'PASSWORD' in var:
                    print(f"  {desc}: {'*' * len(value)}")
                else:
                    print(f"  {desc}: {value}")
            else:
                print(f"  {desc}: 未设置")
                missing_vars.append(var)
        
        if missing_vars:
            print(f"\n❌ 缺少必要的MySQL环境变量: {', '.join(missing_vars)}")
            return False
    else:
        # SQLite配置
        sqlite_path = project_root / "data" / "data.sqlite3"
        print(f"\nSQLite数据库路径: {sqlite_path}")
        if sqlite_path.exists():
            print("✅ SQLite数据库文件已存在")
        else:
            print("ℹ️  SQLite数据库文件不存在，将在首次运行时创建")
    
    # 测试数据库连接
    try:
        print(f"\n测试数据库连接...")
        db_url = get_database_url()
        
        # 隐藏密码
        if 'mysql' in db_url and os.getenv('BILIUP_MYSQL_PASSWORD'):
            safe_url = db_url.replace(os.getenv('BILIUP_MYSQL_PASSWORD'), '***')
        else:
            safe_url = db_url
        print(f"数据库URL: {safe_url}")
        
        # 测试连接
        with engine.connect() as connection:
            if db_type == 'mysql':
                result = connection.execute("SELECT VERSION()")
                version = result.fetchone()[0]
                print(f"MySQL版本: {version}")
            else:
                print("SQLite连接成功")
            
            # 测试表创建
            print("测试表结构...")
            BaseModel.metadata.create_all(engine)
            print("✅ 表结构创建成功")
        
        print("\n✅ 数据库配置检查通过!")
        return True
        
    except Exception as e:
        print(f"\n❌ 数据库连接失败: {e}")
        return False


def main():
    """主函数"""
    success = check_database_config()
    
    if success:
        print("\n建议:")
        print("1. 数据库配置正确，可以正常运行应用")
        print("2. 运行: python -m biliup")
    else:
        print("\n故障排除:")
        if os.getenv('BILIUP_DB_TYPE', 'sqlite').lower() == 'mysql':
            print("1. 检查 MySQL 服务是否运行")
            print("2. 验证用户名和密码")
            print("3. 确认数据库是否存在")
            print("4. 检查网络连接和防火墙设置")
        else:
            print("1. 检查 data 目录权限")
            print("2. 确保有足够的磁盘空间")
        
        sys.exit(1)


if __name__ == "__main__":
    main() 