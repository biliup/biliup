import os
import random
import time

from PIL import Image
from selenium.webdriver.common.action_chains import ActionChains
from selenium.webdriver.common.by import By
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.support.ui import WebDriverWait


class slider_cracker(object):
    def __init__(self, driver):
        self.driver = driver
        self.driver.maximize_window()  # 最大化窗口
        self.driver.set_window_size(1024, 768)
        self.fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'true_image.png')

    def get_true_image(self, slider_xpath=r'//*[@id="gc-box"]/div/div[3]/div[1]'):
        # element = WebDriverWait(self.driver, 50).until(EC.element_to_be_clickable((By.XPATH, slider_xpath)))
        element = WebDriverWait(self.driver, 50).until(
            EC.element_to_be_clickable((By.CLASS_NAME, "geetest_slider_button")))
        ActionChains(self.driver).move_to_element(element).perform()  # 鼠标移动到滑动框以显示图片
        js = 'document.querySelector("body > div.geetest_panel.geetest_wind ' \
             '> div.geetest_panel_box.geetest_no_logo.geetest_panelshowslide ' \
             '> div.geetest_panel_next > div > div.geetest_wrap > div.geetest_widget ' \
             '> div > a > div.geetest_canvas_img.geetest_absolute > canvas").' \
             'style.display = "%s";'
        self.driver.execute_script(js % "inline")
        time.sleep(1)
        true_image = self.get_img(self.fn)
        self.driver.execute_script(js % "none")
        return true_image

    def get_img(self, img_name, img_xpath=r'//*[@id="gc-box"]/div/div[1]/div[2]/div[1]/a[2]'):  # 260*116
        fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'slider_screenshot.png')

        screen_shot = self.driver.save_screenshot(fn)
        # image_element = self.driver.find_element_by_xpath(img_xpath)
        image_element = self.driver.find_element_by_class_name(r'geetest_window')
        left = image_element.location['x']
        top = image_element.location['y']  # selenium截图并获取验证图片location后将其截出保存
        right = image_element.location['x'] + image_element.size['width']
        bottom = image_element.location['y'] + image_element.size['height']
        image = Image.open(fn)
        image = image.crop((left, top, right, bottom))
        image.save(img_name)
        return image

    def analysis(self, true_image, knob_xpath=r'//*[@id="gc-box"]/div/div[3]/div[2]'):
        fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'img2.png')
        img1 = Image.open(self.fn)
        # slider_element = self.driver.find_element_by_xpath(knob_xpath)
        slider_element = self.driver.find_element_by_class_name("geetest_slider_button")
        ActionChains(self.driver).click_and_hold(slider_element).perform()  # 点击滑块后截取残缺图
        time.sleep(1)
        img2 = self.get_img(fn)
        img1_width, img1_height = img1.size
        img2_width, img2_height = img2.size
        left = 0
        flag = False

        for i in range(69, img1_width):  # 遍历x>65的像素点（x<65是拼图块）
            for j in range(0, img1_height):
                if not self.is_pixel_equal(img1, img2, i, j):
                    left = i
                    flag = True
                    break
            if flag:
                break
        if left >= 73:
            left = left - 3  # 误差纠正
        else:
            left = left
        return left

    def is_pixel_equal(self, img1, img2, x, y):  # 通过比较俩图片像素点RGB值差值判断是否为缺口
        pix1 = img1.load()[x, y]
        pix2 = img2.load()[x, y]
        if (abs(pix1[0] - pix2[0] < 60) and abs(pix1[1] - pix2[1] < 60) and abs(pix1[2] - pix2[2] < 60)):
            return True
        else:
            return False

    def get_track(self, distance):
        """
        根据偏移量获取移动轨迹
        :param distance: 偏移量
        :return: 移动轨迹
        """
        # 移动轨迹
        track = []
        # 当前位移
        current = 0
        # 减速阈值
        mid = distance * 4 / 5
        # 计算间隔
        t = 0.2
        # 初速度
        v = 0

        while current < distance:
            if current < mid:
                # 加速度为正2
                a = 2
            else:
                # 加速度为负3
                a = -3
            # 初速度v0
            v0 = v
            # 当前速度v = v0 + at
            v = v0 + a * t
            # 移动距离x = v0t + 1/2 * a * t^2
            move = v0 * t + 1 / 2 * a * t * t
            # 当前位移
            current += move
            # 加入轨迹
            track.append(round(move))
        # print(track)
        #
        # print(sum(track))
        track.append(distance - sum(track))
        # print(track)
        # print(sum(track))
        return track

    def move_to_gap(self, slider, track):
        """
        拖动滑块到缺口处
        :param slider: 滑块
        :param track: 轨迹
        :return:
        """
        ActionChains(self.driver).click_and_hold(slider).perform()
        for x in track:
            ActionChains(self.driver).move_by_offset(xoffset=x, yoffset=random.uniform(-5, 2)).perform()
        time.sleep(0.5)
        ActionChains(self.driver).release().perform()

    def crack(self):
        true_image = self.get_true_image()
        x_offset = self.analysis(true_image)
        print(x_offset)

        track = self.get_track(x_offset)
        knob_element = WebDriverWait(self.driver, 50).until(
            EC.element_to_be_clickable((By.XPATH, r'/html/body/div[2]/div[2]/div[6]/div/div[1]/div[2]/div[2]')))
        self.move_to_gap(knob_element, track)

        # fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'result0.png')
        # fn1 = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'result1.png')
        # time.sleep(0.02)
        # screen_shot = self.driver.save_screenshot(fn)
        # time.sleep(2)
        # screen_shot = self.driver.save_screenshot(fn1)
