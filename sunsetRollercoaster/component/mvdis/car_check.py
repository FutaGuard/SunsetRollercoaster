import ddddocr
from bs4 import BeautifulSoup
from tomlkit import table
from sunsetRollercoaster import crawler
from sunsetRollercoaster.crawler._crawler import Crawler
from typing import Any

class CarCheck(Crawler):
    def __init__(self, proxy: Crawler.Proxy = Crawler.Proxy.JP):
        super().__init__(proxy)

    async def query(self, url):
        return await super().query(url)

    async def check_car(self, queryType: str,companyNo: str='',plateNo: str='',idNo: str='',birthday: str='') -> Any:
        '''
        check_car 查詢車輛定檢日
        :param queryType: 1是個人 2是公司
        :param companyNo: 身分證或是統編
        :param plateNo: 車牌號碼
        '''
        #
        url ='https://www.mvdis.gov.tw/m3-emv-car/car/checkQuery'
        resp = await self.client.get(url)

        captcha_img = await self.client.get('https://www.mvdis.gov.tw//m3-emv-car/captchaImg.jpg')
        ocr = ddddocr.DdddOcr(show_ad=False)
        captcha_code = ocr.classification(captcha_img.content)

        data = {
            'method': 'queryCheck',
            'queryType': queryType,
            'queryMode': '1',
            'companyNo': companyNo,
            'plateNo': plateNo,
            'idNo': idNo,
            'birthday': birthday,
            'perPlateNo': '',
            'carType': '1',
            'validateStr': captcha_code,
        }
        resp = await self.client.post('https://www.mvdis.gov.tw/m3-emv-car/car/checkQuery', data=data)
        soup = BeautifulSoup(resp.text, 'html.parser')
        table = soup.find('table', id='info')
        rows = table.find('tbody').find_all('tr')

        results = []  # 用來存放所有查詢結果
        for row in rows:
            cols = row.find_all('td')
            data = [col.get_text(strip=True) for col in cols]
            result_dict = {
                '車種': data[0],
                '車號': data[1],
                '下次定檢日': data[2]
            }
            results.append(result_dict)

        return results